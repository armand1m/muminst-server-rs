use std::path::Path;

use actix_web::{
    get, post,
    web::{Data, Json},
    Error, HttpResponse,
};
use serde::{Deserialize, Serialize};
use serenity::model::id::{ChannelId, GuildId};

use crate::app_state::AppState;

use songbird::{
    driver::Bitrate,
    input::{self, cached::Compressed},
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
pub struct PlaySoundErrorPayload {
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaySoundResponse {
    sound_id: String,
    client: Client,
}

#[get("/")]
pub async fn index(data: Data<AppState>) -> Result<String, Error> {
    let app_name = &data.app_name; // <- get app_name
    let ctx_mutex = &data.discord_ctx.try_lock().unwrap();
    let context = ctx_mutex.as_ref().unwrap();

    // #guests channel
    let channel = ChannelId(641453061608439819);
    let _ = channel
        .send_message(&context, |m| m.embed(|e| e.title("It works")))
        .await;

    Ok(format!("Hello {}!", app_name))
}

#[post("/play-sound")]
pub async fn play_sound(
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

        let audio_folder_path = Path::new("data/audio");

        // TODO: Check if file exist before sending it to ffmpeg
        // For some reason the ffmpeg does not check for that
        let sound_src = Compressed::new(
            input::ffmpeg(audio_folder_path.join("ecbcecb6-e82b-4aeb-8716-8f39b0446d36.mp3"))
                .await
                .expect("Link may be dead."),
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
