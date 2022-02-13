use actix_web::{
    get,
    web::{self, Data},
    Error, HttpResponse,
};
use serde::Serialize;

use crate::{actions::sounds::fetch_sounds_with_tags, app_state::AppState};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorPayload {
    message: String,
}

#[get("/sounds")]
pub async fn sounds_handler(data: Data<AppState>) -> Result<HttpResponse, Error> {
    let result = web::block(move || {
        let database_connection = &data
            .database_pool
            .get()
            .expect("couldn't get db connection from pool");

        fetch_sounds_with_tags(database_connection)
    })
    .await?;

    let response = match result {
        Ok(sounds) => sounds,
        Err(reason) => {
            eprintln!("Failed to fetch sounds from database. Reason: {:?}", reason);
            return Ok(HttpResponse::InternalServerError().json(ErrorPayload {
                message: "Server failed to fetch sounds from database.".to_string(),
            }));
        }
    };

    Ok(HttpResponse::Ok().json(response))
}
