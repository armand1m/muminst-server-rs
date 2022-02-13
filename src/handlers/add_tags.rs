use crate::{actions::tags::insert_tags, app_state::AppState};
use actix_web::{
    put,
    web::{Data, Json, Path},
    Error, HttpResponse,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct AddTagsRequestPath {
    sound_id: String,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AddTagsRequestBody {
    tags: Vec<String>,
}

#[put("/add-tags/{sound_id}")]
pub async fn add_tags_handler(
    path: Path<AddTagsRequestPath>,
    body: Json<AddTagsRequestBody>,
    data: Data<AppState>,
) -> Result<HttpResponse, Error> {
    let updated_sound = insert_tags(
        path.sound_id.clone(),
        body.tags.clone(),
        data.database_pool.clone(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(updated_sound))
}
