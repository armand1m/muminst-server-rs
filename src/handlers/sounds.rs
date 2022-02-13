use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
};

use actix_web::{
    get,
    web::{self, Data},
    Error, HttpResponse,
};
use serde::Serialize;

use crate::{
    app_state::AppState,
    models::{Sound, SoundWithTags, Tag},
    schema::sounds,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorPayload {
    message: String,
}

fn fetch_sounds(
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
