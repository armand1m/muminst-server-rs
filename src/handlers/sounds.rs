use std::path::Path;

use actix_web::{
    post,
    web::{Data, Json},
    Error, HttpResponse,
};
use serde::{Deserialize, Serialize};
use serenity::model::id::GuildId;
use songbird::{
    driver::Bitrate,
    input::{self, cached::Compressed},
};

use crate::{app_state::AppState, storage::get_audio_path};

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
pub struct PlaySoundErrorPayload {
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaySoundResponse {
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

    let guild_id: GuildId = GuildId {
        // server da maconha guild id
        0: 194951764045201409,
    };

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        // TODO: fetch audio folder path from env var
        let audio_folder_path = Path::new("data/audio");
        let audio_path = get_audio_path(audio_folder_path, json.sound_id.clone());

        if !audio_path.exists() {
            return Ok(
                HttpResponse::InternalServerError().json(PlaySoundErrorPayload {
                    message: format!("Audio is missing for sound with id: {}", json.sound_id),
                }),
            );
        }

        let sound_src = Compressed::new(
            input::ffmpeg(audio_path).await.expect("Link may be dead."),
            Bitrate::BitsPerSecond(128_000),
        )
        .expect("ffmpeg parameters to be properly defined");

        let track = handler.play_source(sound_src.into());
        let _ = track.set_volume(0.8);
    } else {
        return Ok(HttpResponse::BadRequest().json(PlaySoundErrorPayload {
            message: "Bot has to join a voice channel first.".to_string(),
        }));
    }

    Ok(HttpResponse::Ok().json(PlaySoundResponse {
        sound_id: json.sound_id.to_owned(),
        client: json.client.to_owned(),
    }))
}
