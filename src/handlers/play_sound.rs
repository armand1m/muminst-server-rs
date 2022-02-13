use std::path::Path;

use actix_web::{
    post,
    web::{self, Data, Json},
    Error, HttpResponse,
};
use serde::{Deserialize, Serialize};
use serenity::model::id::GuildId;
use songbird::{
    driver::Bitrate,
    input::{self, cached::Compressed},
};

use crate::{actions::sounds::fetch_sound_by_id, app_state::AppState};

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
struct ErrorPayload {
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaySoundResponse {
    sound_id: String,
    client: Client,
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

            fetch_sound_by_id(sound_id, database_connection)
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
