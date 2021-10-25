use std::io::Error;

use actix_web::{get, web::Data};
use serenity::model::id::ChannelId;

use crate::app_state::AppState;

#[get("/")]
pub async fn index(data: Data<AppState>) -> Result<String, Error> {
    let app_name = &data.app_name; // <- get app_name
    let ctx_mutex = &data.discord_ctx.try_lock().unwrap();
    let context = ctx_mutex.as_ref().unwrap();

    let _ = ChannelId(641453061608439819) // #guests channel
        .send_message(&context, |m| m.embed(|e| e.title("It works")))
        .await;

    Ok(format!("Hello {}!", app_name))
}
