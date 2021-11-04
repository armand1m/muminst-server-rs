use std::sync::{Arc, Mutex};

use diesel::SqliteConnection;
use serenity::client::Context;

pub struct AppState {
    pub app_name: String,
    pub discord_ctx: Arc<Mutex<Option<Context>>>,
    pub database_connection: SqliteConnection,
    pub audio_folder_path: String,
}
