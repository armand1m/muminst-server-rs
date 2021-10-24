use actix_web::{get, web::Data};

use crate::app_state::AppState;

#[get("/")]
pub async fn index(data: Data<AppState>) -> String {
    let app_name = &data.app_name; // <- get app_name
    format!("Hello {}!", app_name) // <- response with app_name
}
