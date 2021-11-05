use actix_multipart::{Field, Multipart};
use diesel::{insert_into, prelude::*};
use std::{
    fs::File,
    io::{Cursor, ErrorKind, Write},
    path::Path,
};
use uuid::Uuid;

use actix_web::{
    get, post,
    web::{self, Data, Json},
    Error, HttpResponse,
};
use serde::{Deserialize, Serialize};
use serenity::{futures::TryStreamExt, model::id::GuildId};
use sha2::{Digest, Sha512};
use songbird::{
    driver::Bitrate,
    input::{self, cached::Compressed},
};

use crate::{app_state::AppState, models::Sound, schema::sounds};
use crate::{app_state::DatabasePool, schema::sounds::dsl::sounds as sounds_dsl};

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

#[get("/sounds")]
pub async fn sounds_handler(data: Data<AppState>) -> Result<HttpResponse, Error> {
    // TODO: fetch tags as well, add left_join
    let query_result = web::block(move || {
        let query = sounds::table;
        let database_connection = &data
            .database_pool
            .get()
            .expect("couldn't get db connection from pool");
        query.load::<Sound>(database_connection)
    })
    .await?;

    let sounds = match query_result {
        Ok(sounds) => sounds
            .into_iter()
            .map(|x| Sound {
                extension: format!(".{}", x.extension),
                file_name: x.file_name,
                file_hash: x.file_hash,
                id: x.id,
                name: x.name,
            })
            .collect::<Vec<Sound>>(),
        Err(reason) => {
            eprintln!("Failed to fetch sounds from database. Reason: {:?}", reason);
            return Ok(HttpResponse::InternalServerError().json(ErrorPayload {
                message: "Server failed to fetch sounds from database.".to_string(),
            }));
        }
    };

    Ok(HttpResponse::Ok().json(sounds))
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
        let track = handler.play_source(sound_src.into());
        let _ = track.set_volume(1.0);
    } else {
        return Ok(HttpResponse::BadRequest().json(ErrorPayload {
            message: "Bot has to join a voice channel first.".to_string(),
        }));
    }

    Ok(HttpResponse::Ok().json(PlaySoundResponse {
        sound_id: json.sound_id.to_owned(),
        client: json.client.to_owned(),
    }))
}

async fn validate_sound(
    field: &mut Field,
) -> Result<(Cursor<Vec<u8>>, infer::Type, String), Error> {
    /*
     * Creates a Cursor to load the buffer
     * in memory before actually writing it
     * to the disk
     */
    let mut memory_file = Cursor::new(Vec::<u8>::new());

    while let Some(chunk) = field.try_next().await? {
        // filesystem operations are blocking,
        // so we have to use a threadpool
        memory_file = web::block(move || memory_file.write_all(&chunk).map(|_| memory_file))
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
     * Create a SHA512 hash from the buffer
     */
    let mut hasher = Sha512::new();
    hasher.update(memory_file_buf);
    let hash_result = hasher.finalize();
    let file_hash = format!("{:x}", hash_result);

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
    field: &mut Field,
    payload_filename: &str,
    audio_folder_path: &Path,
    database_pool: DatabasePool,
) -> Result<UploadSuccess, Error> {
    let original_filename = Path::new(payload_filename);
    let (memory_file, file_type, file_hash) = validate_sound(field).await?;

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

    let insertable = sound_record.clone();
    web::block(move || {
        let database_connection = database_pool
            .get()
            .expect("Failed to get db connection from db pool");

        insert_into(sounds_dsl)
            .values(&insertable)
            .execute(&database_connection)
            .expect("Failed to insert sound in database.");
    })
    .await?;

    Ok(UploadSuccess {
        id: sound_record.id.clone(),
        filename: payload_filename.to_string(),
    })
}

#[post("/upload")]
async fn upload_handler(
    mut payload: Multipart,
    data: Data<AppState>,
) -> Result<HttpResponse, Error> {
    let audio_folder_path = Path::new(&data.audio_folder_path);

    let mut successful_uploads: Vec<UploadSuccess> = vec![];
    let mut failed_uploads: Vec<UploadFailure> = vec![];

    while let Some(mut field) = payload.try_next().await? {
        // A multipart/form-data stream has to contain `content_disposition`
        // this is where we'll be able to fetch the file name and
        // the key name for other parts of the payload
        let content_disposition = field
            .content_disposition()
            .expect("Content disposition to be present");

        let field_key = content_disposition
            .get_name()
            .expect("Content disposition to have a name");

        if field_key == "tags" {
            // TODO: insert tags to database
            // Assignee: @shayronaguiar
            break;
        }

        let payload_filename = content_disposition
            .get_filename()
            .expect("Uploaded file to always contain a filename.");

        let database_pool = data.database_pool.clone();
        let upload_result = upload_payload_file(
            &mut field,
            payload_filename,
            audio_folder_path,
            database_pool,
        )
        .await;

        match upload_result {
            Ok(successful) => successful_uploads.push(successful),
            Err(failure) => failed_uploads.push(UploadFailure {
                filename: payload_filename.to_string(),
                reason: failure.to_string(),
            }),
        }
    }

    Ok(HttpResponse::Ok().json(UploadResponse {
        successful: successful_uploads,
        failed: failed_uploads,
        tags: vec![],
    }))
}
