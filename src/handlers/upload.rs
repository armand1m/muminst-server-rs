use actix_multipart::{Field, Multipart};
use serde::Serialize;
use serenity::futures::{StreamExt, TryStreamExt};
use uuid::Uuid;

use std::{future::Future, io::ErrorKind, path::Path, pin::Pin};

use actix_web::{
    dev::Payload,
    post,
    web::{self, Bytes, Data},
    Error, FromRequest, HttpRequest, HttpResponse,
};

use crate::{
    actions::{
        fs::{save_sound_as_file, validate_sound},
        sounds::{fetch_sound_by_hash, insert_sound},
    },
    app_state::{AppState, DatabasePool},
    models::Sound,
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
    let sound_hash_match = web::block(move || {
        let database_connection = database_pool_clone
            .get()
            .expect("failed to acquire db connection from db pool");

        fetch_sound_by_hash(file_hash_clone, &database_connection)
    })
    .await?;

    if sound_hash_match.is_some() {
        return Err(std::io::Error::new(ErrorKind::AlreadyExists, "File already exists").into());
    }

    let extension = file_type.extension();
    let file_name = Uuid::new_v4().to_string();

    let _ = save_sound_as_file(
        memory_file,
        file_name.clone(),
        extension.to_string(),
        audio_folder_path,
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

    let insertable = sound_record.clone();
    web::block(move || {
        let database_connection = database_pool
            .get()
            .expect("Failed to get db connection from db pool");

        insert_sound(insertable, slugs, &database_connection);
    })
    .await?;

    Ok(UploadSuccess {
        id: sound_record.id.clone(),
        filename: filename.to_string(),
    })
}
