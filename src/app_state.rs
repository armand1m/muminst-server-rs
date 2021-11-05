use std::sync::{Arc, Mutex};

use diesel::{
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};
use serenity::client::Context;

pub type DatabasePool = Pool<ConnectionManager<SqliteConnection>>;

pub struct AppState {
    pub app_name: String,
    pub discord_ctx: Arc<Mutex<Option<Context>>>,
    pub discord_guild_id: u64,
    pub database_pool: DatabasePool,
    pub audio_folder_path: String,
}
