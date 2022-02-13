use actix_web::{web, Error};
use diesel::{insert_into, prelude::*};
use uuid::Uuid;

use crate::{
    app_state::DatabasePool,
    models::{SoundWithTags, Tag},
};

use crate::{actions::sounds::fetch_sound_by_id, schema::tags::dsl::tags as tags_dsl};

pub async fn insert_tags(
    sound_id: String,
    slugs: Vec<String>,
    database_pool: DatabasePool,
) -> Result<SoundWithTags, Error> {
    let tag_records = slugs
        .into_iter()
        .map(|slug| Tag {
            sound_id: sound_id.clone(),
            id: Uuid::new_v4().to_string(),
            slug,
        })
        .collect::<Vec<_>>();

    let sound = web::block(move || {
        let database_connection = database_pool
            .get()
            .expect("Failed to get db connection from db pool");

        insert_into(tags_dsl)
            .values(tag_records)
            // https://github.com/diesel-rs/diesel/issues/1822
            .execute(&*database_connection)
            .expect("Failed to insert tags in database.");

        fetch_sound_by_id(sound_id, &database_connection)
    })
    .await?;

    Ok(sound.unwrap())
}
