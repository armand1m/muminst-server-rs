use crate::schema::{sounds, tags};

use serde::{Deserialize, Serialize};
#[derive(Queryable, Associations, Identifiable, Deserialize, Serialize, Insertable)]
#[serde(rename_all = "camelCase")]
pub struct Sound {
    pub id: String,
    pub name: String,
    pub extension: String,
    pub file_name: String,
    pub file_hash: String,
}

#[derive(Queryable, Associations, Identifiable, Deserialize, Serialize, Insertable)]
#[serde(rename_all = "camelCase")]
#[belongs_to(Sound)]
pub struct Tag {
    pub id: String,
    pub sound_id: String,
    pub slug: String,
}
