use actix_multipart::{Field, Multipart};
use diesel::{insert_into, prelude::*};
use std::{
    fs::File,
    future::Future,
    io::{Cursor, ErrorKind, Write},
    path::Path,
    pin::Pin,
};
use uuid::Uuid;

use actix_web::{
    dev::Payload,
    post,
    web::{self, Bytes, Data},
    Error, FromRequest, HttpRequest, HttpResponse,
};
use serde::Serialize;
use serenity::futures::{StreamExt, TryStreamExt};
use sha256::digest_bytes;

use crate::{
    app_state::AppState,
    models::{Sound, Tag},
    schema::sounds,
};

use crate::{
    app_state::DatabasePool, schema::sounds::dsl::sounds as sounds_dsl,
    schema::tags::dsl::tags as tags_dsl,
};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct UploadSuccess {
    id: String,
    filename: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct UploadFailure {
    filename: String,
    reason: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct UploadResponse {
    successful: Vec<UploadSuccess>,
    failed: Vec<UploadFailure>,
    tags: Vec<String>,
}

// https://gist.github.com/Tarkin25/b6274a8a33baa6a72d7763e298f1fb8f
struct SoundUpload {
    filename: String,
    file_content: Vec<Bytes>,
}

pub struct BatchSoundUpload {
    sounds: Vec<SoundUpload>,
    tags: Vec<String>,
}

impl BatchSoundUpload {
    async fn read_string(field: &mut Field) -> Option<String> {
        let bytes = field.try_next().await;

        if let Ok(Some(bytes)) = bytes {
            String::from_utf8(bytes.to_vec()).ok()
        } else {
            None
        }
    }

    async fn from_multipart(
        mut multipart: Multipart,
    ) -> Result<Self, <Self as FromRequest>::Error> {
        let mut tags: Vec<String> = Vec::new();
        let mut sounds: Vec<SoundUpload> = Vec::new();

        while let Some(mut field) = multipart.try_next().await? {
            // A multipart/form-data stream has to contain `content_disposition`
            // this is where we'll be able to fetch the file name and
            // the key name for other parts of the payload
            let content_disposition = field.content_disposition().clone();

            let field_key = content_disposition
                .get_name()
                .expect("Content disposition to have a name");

            if field_key == "tags" {
                let field_content = Self::read_string(&mut field).await;

                if let Some(json_content) = field_content {
                    tags = serde_json::from_str(&json_content)?;
                }

                break;
            }

            let filename = content_disposition
                .get_filename()
                .expect("Uploaded file to always contain a filename.");

            let sound_upload = SoundUpload {
                filename: filename.to_string(),
                file_content: field
                    .map(|chunk| chunk.unwrap())
                    .collect::<Vec<Bytes>>()
                    .await,
            };

            sounds.push(sound_upload);
        }

        Ok(BatchSoundUpload { tags, sounds })
    }
}

impl FromRequest for BatchSoundUpload {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        // get a future for a Multipart struct from the request
        let multipart_future = Multipart::from_request(req, payload);

        // As this is not an async function, we cannot use 'await'.
        // Instead, we create future from this async block and return a pinned Box containing our future.
        // This is because currently, traits cannot declare async functions, so instead the FromRequest trait declares a non-async function which returns a Future instead.
        let future = async {
            // Inside of this async block we are able to use 'await'
            let multipart = multipart_future.await?;

            // Await our async function containing the actual logic
            Self::from_multipart(multipart).await
        };

        Box::pin(future)
    }
}

#[post("/upload")]
pub async fn upload_handler(
    payload: BatchSoundUpload,
    data: Data<AppState>,
) -> Result<HttpResponse, Error> {
    let audio_folder_path = Path::new(&data.audio_folder_path);
    let mut successful_uploads: Vec<UploadSuccess> = vec![];
    let mut failed_uploads: Vec<UploadFailure> = vec![];

    for sound_upload in payload.sounds.into_iter() {
        let database_pool = data.database_pool.clone();
        let upload_result = upload_payload_file(
            sound_upload.file_content,
            &sound_upload.filename,
            audio_folder_path,
            database_pool,
            payload.tags.clone(),
        )
        .await;

        match upload_result {
            Ok(successful) => successful_uploads.push(successful),
            Err(failure) => failed_uploads.push(UploadFailure {
                filename: sound_upload.filename.to_string(),
                reason: failure.to_string(),
            }),
        }
    }

    Ok(HttpResponse::Ok().json(UploadResponse {
        successful: successful_uploads,
        failed: failed_uploads,
        tags: payload.tags,
    }))
}

async fn upload_payload_file(
    file_content: Vec<Bytes>,
    filename: &str,
    audio_folder_path: &Path,
    database_pool: DatabasePool,
    slugs: Vec<String>,
) -> Result<UploadSuccess, Error> {
    let original_filename = Path::new(filename);
    let (memory_file, file_type, file_hash) = validate_sound(file_content).await?;

    let file_hash_clone = file_hash.clone();
    let database_pool_clone = database_pool.clone();
    let hash_query = web::block(move || {
        let database_connection = database_pool_clone
            .get()
            .expect("failed to acquire db connection from db pool");

        sounds::table
            .filter(sounds::file_hash.eq(&file_hash_clone))
            .first::<Sound>(&database_connection)
            .optional()
            .expect("Failed to query by hash")
    })
    .await?;

    if let Some(_) = hash_query {
        return Err(std::io::Error::new(ErrorKind::AlreadyExists, "File already exists").into());
    }

    let extension = file_type.extension();
    let file_name = Uuid::new_v4().to_string();

    let _ = save_sound_to(
        memory_file,
        audio_folder_path,
        file_name.clone(),
        extension.to_string(),
    )
    .await?;

    let sound_name = original_filename
        .file_stem()
        .expect("Filename to have a basename defined")
        .to_str()
        .unwrap()
        .to_string();

    let sound_record = Sound {
        id: Uuid::new_v4().to_string(),
        name: sound_name,
        file_name,
        file_hash,
        extension: extension.to_string(),
    };

    let tag_records = slugs
        .into_iter()
        .map(|slug| Tag {
            sound_id: sound_record.id.clone(),
            id: Uuid::new_v4().to_string(),
            slug,
        })
        .collect::<Vec<_>>();

    let insertable = sound_record.clone();
    web::block(move || {
        let database_connection = database_pool
            .get()
            .expect("Failed to get db connection from db pool");

        insert_into(sounds_dsl)
            .values(&insertable)
            .execute(&database_connection)
            .expect("Failed to insert sound in database.");

        insert_into(tags_dsl)
            .values(tag_records)
            // https://github.com/diesel-rs/diesel/issues/1822
            .execute(&*database_connection)
            .expect("Failed to insert tags in database.");
    })
    .await?;

    Ok(UploadSuccess {
        id: sound_record.id.clone(),
        filename: filename.to_string(),
    })
}

async fn validate_sound(
    file_content: Vec<Bytes>,
) -> Result<(Cursor<Vec<u8>>, infer::Type, String), Error> {
    /*
     * Creates a Cursor to load the buffer
     * in memory before actually writing it
     * to the disk
     */
    let mut memory_file = Cursor::new(Vec::<u8>::new());
    let mut content_iter = file_content.to_owned().into_iter();

    while let Some(chunk) = content_iter.next() {
        memory_file = web::block(move || memory_file.write_all(&chunk).map(|_| memory_file))
            .await
            .expect("File content to be written")?;
    }

    let memory_file_buf = memory_file.get_ref();
    let file_type_slice = &memory_file_buf[0..4];
    let file_type = match infer::get(file_type_slice) {
        Some(file_type) => file_type,
        None => {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                "Failed to identify the file type.",
            )
            .into());
        }
    };

    let file_extension = file_type.extension();
    let valid_extensions = ["mp3", "wav", "ogg", "webm"];

    if !valid_extensions.contains(&file_extension) {
        return Err(std::io::Error::new(ErrorKind::InvalidData, "File type is not valid").into());
    }

    /*
     * Create a SHA256 hash from the buffer
     */
    let file_hash = digest_bytes(memory_file_buf);

    Ok((memory_file, file_type, file_hash))
}

async fn save_sound_to(
    memory_file: Cursor<Vec<u8>>,
    audio_folder_path: &Path,
    file_name: String,
    extension: String,
) -> Result<File, Error> {
    let mut filepath = audio_folder_path.join(&file_name);
    filepath.set_extension(extension);

    /*
     * Load buffer from memory file into the
     * actual filesystem file
     */
    let file = web::block(move || {
        let mut file = File::create(filepath).expect("File to be created");
        file.write_all(&memory_file.get_ref())
            .expect("File content to be written to filesystem");
        file
    })
    .await
    .expect("File content to be written to filesystem");

    Ok(file)
}
