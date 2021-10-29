use std::io::Error;
use std::io::ErrorKind;

use actix_web::{get, web::Data};
use serenity::model::id::ChannelId;
use serenity::model::id::GuildId;

use crate::app_state::AppState;

use songbird::{
    driver::Bitrate,
    input::{self, cached::Compressed},
};

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

#[get("/play-sound")]
pub async fn play_sound(data: Data<AppState>) -> Result<String, Error> {
    let ctx_mutex = &data.discord_ctx.try_lock().unwrap();
    let ctx = ctx_mutex.as_ref().unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let guild_id: GuildId = GuildId {
        // server da maconha guild id
        // en
        0: 194951764045201409,
    };

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        let song_src = Compressed::new(
            input::ffmpeg("data/audio/ecbcecb6-e82b-4aeb-8716-8f39b0446d36.mp3")
                .await
                .expect("Link may be dead."),
            Bitrate::BitsPerSecond(128_000),
        )
        .expect("These parameters are well-defined.");

        let song = handler.play_source(song_src.into());
        let _ = song.set_volume(0.8);
    } else {
        return Err(Error::new(ErrorKind::NotConnected, "Join a channel first"));
    }

    Ok(String::from("Sound is playing"))
}
