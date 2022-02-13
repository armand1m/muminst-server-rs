use crate::schema::{sounds, tags};

use diesel::Queryable;
use serde::{Deserialize, Serialize};
#[derive(Queryable, Associations, Identifiable, Deserialize, Serialize, Insertable, Clone)]
#[table_name = "sounds"]
#[serde(rename_all = "camelCase")]
pub struct Sound {
    pub id: String,
    pub name: String,
    pub extension: String,
    pub file_name: String,
    pub file_hash: String,
}

#[derive(Queryable, Associations, Identifiable, Deserialize, Serialize, Insertable, Clone)]
#[serde(rename_all = "camelCase")]
#[table_name = "tags"]
#[belongs_to(Sound)]
pub struct Tag {
    pub id: String,
    pub sound_id: String,
    pub slug: String,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SoundWithTags {
    pub id: String,
    pub name: String,
    pub extension: String,
    pub file_name: String,
    pub file_hash: String,
    pub tags: Vec<String>,
}
