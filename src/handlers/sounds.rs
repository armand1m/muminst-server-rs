use actix_multipart::{Field, Multipart};
use diesel::{
    insert_into,
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
};
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
    get, post,
    web::{self, Bytes, Data, Json},
    Error, FromRequest, HttpRequest, HttpResponse,
};
use serde::{Deserialize, Serialize};
use serenity::{
    futures::{StreamExt, TryStreamExt},
    model::id::GuildId,
};
use sha256::digest_bytes;
use songbird::{
    driver::Bitrate,
    input::{self, cached::Compressed},
};

use crate::{
    app_state::AppState,
    models::{Sound, SoundWithTags, Tag},
    schema::sounds,
};
use crate::{
    app_state::DatabasePool, schema::sounds::dsl::sounds as sounds_dsl,
    schema::tags::dsl::tags as tags_dsl,
};

#[derive(Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
enum Client {
    Discord,
    Telegram,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaySoundPayload {
    sound_id: String,
    client: Client,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorPayload {
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaySoundResponse {
    sound_id: String,
    client: Client,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UploadSuccess {
    id: String,
    filename: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UploadFailure {
    filename: String,
    reason: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UploadResponse {
    successful: Vec<UploadSuccess>,
    failed: Vec<UploadFailure>,
    tags: Vec<String>,
}

pub fn fetch_sounds(
    database_connection: &PooledConnection<ConnectionManager<SqliteConnection>>,
) -> Result<Vec<(Sound, Vec<Tag>)>, diesel::result::Error> {
    let sounds = sounds::table.load::<Sound>(database_connection)?;
    let tags = Tag::belonging_to(&sounds)
        .load::<Tag>(database_connection)?
        .grouped_by(&sounds);

    let data = sounds.into_iter().zip(tags).collect::<Vec<_>>();
    Ok(data)
}

#[get("/sounds")]
pub async fn sounds_handler(data: Data<AppState>) -> Result<HttpResponse, Error> {
    let result = web::block(move || {
        let database_connection = &data
            .database_pool
            .get()
            .expect("couldn't get db connection from pool");

        fetch_sounds(database_connection)
    })
    .await?;

    let response = match result {
        Ok(sounds) => sounds
            .into_iter()
            .map(|(x, tags)| SoundWithTags {
                extension: format!(".{}", x.extension),
                file_name: x.file_name,
                file_hash: x.file_hash,
                id: x.id,
                name: x.name,
                tags: tags.into_iter().map(|tag| tag.slug).collect(),
            })
            .collect::<Vec<SoundWithTags>>(),
        Err(reason) => {
            eprintln!("Failed to fetch sounds from database. Reason: {:?}", reason);
            return Ok(HttpResponse::InternalServerError().json(ErrorPayload {
                message: "Server failed to fetch sounds from database.".to_string(),
            }));
        }
    };

    Ok(HttpResponse::Ok().json(response))
}

#[post("/play-sound")]
pub async fn play_sound_handler(
    data: Data<AppState>,
    json: Json<PlaySoundPayload>,
) -> Result<HttpResponse, Error> {
    let ctx_mutex = data.discord_ctx.try_lock().unwrap();
    let ctx = ctx_mutex.as_ref().unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let guild_id: GuildId = data.discord_guild_id.into();

    if let Some(handler_lock) = manager.get(guild_id) {
        let audio_folder_path = Path::new(&data.audio_folder_path);
        let data_clone = data.clone();
        let sound_id = json.sound_id.clone();

        let sound = web::block(move || {
            let database_connection = &data_clone
                .database_pool
                .get()
                .expect("couldn't get db connection from pool");

            sounds::table
                .filter(sounds::id.eq(sound_id))
                .first::<Sound>(database_connection)
                .optional()
                .expect("Failed to query by sound_id")
        })
        .await?;

        let audio_path = match sound {
            Some(sound) => {
                let mut path = audio_folder_path.join(sound.file_name);
                path.set_extension(sound.extension);
                path
            }
            None => {
                return Ok(HttpResponse::ExpectationFailed().json(ErrorPayload {
                    message: format!("Failed to find sound with id: {}", json.sound_id),
                }));
            }
        };

        if !audio_path.exists() {
            return Ok(HttpResponse::InternalServerError().json(ErrorPayload {
                message: format!("Audio is missing for sound with id: {}", json.sound_id),
            }));
        }

        let audio_source = input::ffmpeg(audio_path).await.expect("Link may be dead.");
        let sound_src = Compressed::new(audio_source, Bitrate::BitsPerSecond(48_000))
            .expect("ffmpeg parameters to be properly defined");

        let mut handler = handler_lock.lock().await;
        handler.play_source(sound_src.into());

        println!("Playing audio: {}", json.sound_id);
    } else {
        return Ok(HttpResponse::BadRequest().json(ErrorPayload {
            message: "Bot has to join a voice channel first.".to_string(),
        }));
    }

    Ok(HttpResponse::Ok().json(PlaySoundResponse {
        sound_id: json.sound_id.clone(),
        client: json.client.clone(),
    }))
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
        let as_slice = chunk.as_ref().to_owned();
        memory_file = web::block(move || memory_file.write_all(&as_slice).map(|_| memory_file))
            .await
            .expect("File content to be written")?;
    }

    let memory_file_buf = memory_file.get_ref();
    let file_type = match infer::get(&memory_file_buf[0..4]) {
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

// https://gist.github.com/Tarkin25/b6274a8a33baa6a72d7763e298f1fb8f
struct SoundUpload {
    filename: String,
    content: Vec<Bytes>,
}
struct BatchSoundUpload {
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

            let payload_filename = content_disposition
                .get_filename()
                .expect("Uploaded file to always contain a filename.");

            let sound_upload = SoundUpload {
                filename: payload_filename.to_string(),
                content: field
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
async fn upload_handler(
    payload: BatchSoundUpload,
    data: Data<AppState>,
) -> Result<HttpResponse, Error> {
    let audio_folder_path = Path::new(&data.audio_folder_path);
    let mut successful_uploads: Vec<UploadSuccess> = vec![];
    let mut failed_uploads: Vec<UploadFailure> = vec![];

    for sound_upload in payload.sounds.into_iter() {
        let database_pool = data.database_pool.clone();
        let upload_result = upload_payload_file(
            sound_upload.content,
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
