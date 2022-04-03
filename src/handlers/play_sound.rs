use std::path::Path;

use actix_web::{
    post,
    web::{self, Data, Json},
    Error, HttpResponse,
};
use serde::{Deserialize, Serialize};

use crate::{actions::sounds::fetch_sound_by_id, app_state::AppState, discord::actor::PlayAudio};

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

    data.discord_actor_addr
        .send(PlayAudio { audio_path })
        .await
        .expect("Failed to play audio");

    Ok(HttpResponse::Ok().json(PlaySoundResponse {
        sound_id: json.sound_id.clone(),
        client: json.client.clone(),
    }))
}
