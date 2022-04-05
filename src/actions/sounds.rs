use diesel::{insert_into, prelude::*};
use uuid::Uuid;

use crate::{
    models::{Sound, SoundWithTags, Tag},
    schema::sounds,
    schema::sounds::dsl::sounds as sounds_dsl,
    schema::tags::dsl::tags as tags_dsl,
};

pub fn fetch_sounds_with_tags(
    database_connection: &SqliteConnection,
) -> Result<Vec<SoundWithTags>, diesel::result::Error> {
    let sounds = sounds::table.load::<Sound>(database_connection)?;
    let tags = Tag::belonging_to(&sounds)
        .load::<Tag>(database_connection)?
        .grouped_by(&sounds);

    let data = sounds.into_iter().zip(tags);
    let sounds = data
        .into_iter()
        .map(|(x, tags)| SoundWithTags {
            extension: format!(".{}", x.extension),
            file_name: x.file_name,
            file_hash: x.file_hash,
            id: x.id,
            name: x.name,
            tags: tags.into_iter().map(|tag| tag.slug).collect(),
        })
        .collect::<Vec<SoundWithTags>>();

    Ok(sounds)
}

pub fn fetch_sound_by_id(
    sound_id: String,
    database_connection: &SqliteConnection,
) -> Option<Sound> {
    sounds::table
        .filter(sounds::id.eq(sound_id))
        .first::<Sound>(database_connection)
        .optional()
        .expect("Failed to query by sound_id")
}

pub fn fetch_sound_with_tags_by_id(
    sound_id: String,
    database_connection: &SqliteConnection,
) -> Option<SoundWithTags> {
    let query_result = sounds::table
        .filter(sounds::id.eq(sound_id))
        .first::<Sound>(database_connection)
        .optional()
        .expect("Failed to query by sound_id");

    query_result.as_ref()?;

    let sound = query_result.unwrap();
    let tags = Tag::belonging_to(&sound)
        .load::<Tag>(database_connection)
        .expect("Failed to fetch tags");

    Some(SoundWithTags {
        extension: format!(".{}", sound.extension),
        file_name: sound.file_name,
        file_hash: sound.file_hash,
        id: sound.id,
        name: sound.name,
        tags: tags.into_iter().map(|tag| tag.slug).collect(),
    })
}

pub fn fetch_sound_by_hash(
    file_hash: String,
    database_connection: &SqliteConnection,
) -> Option<Sound> {
    sounds::table
        .filter(sounds::file_hash.eq(&file_hash))
        .first::<Sound>(database_connection)
        .optional()
        .expect("Failed to query by hash")
}

pub fn insert_sound(sound: Sound, slugs: Vec<String>, database_connection: &SqliteConnection) {
    let tag_records = slugs
        .into_iter()
        .map(|slug| Tag {
            sound_id: sound.id.clone(),
            id: Uuid::new_v4().to_string(),
            slug,
        })
        .collect::<Vec<_>>();

    insert_into(sounds_dsl)
        .values(sound)
        .execute(database_connection)
        .expect("Failed to insert sound in database.");

    insert_into(tags_dsl)
        .values(tag_records)
        // https://github.com/diesel-rs/diesel/issues/1822
        .execute(database_connection)
        .expect("Failed to insert tags in database.");
}
