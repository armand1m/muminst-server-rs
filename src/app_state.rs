use actix::Addr;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};

use crate::discord::actor::DiscordActor;

pub type DatabasePool = Pool<ConnectionManager<SqliteConnection>>;

pub struct AppState {
    pub app_name: String,
    pub discord_actor_addr: Addr<DiscordActor>,
    pub discord_guild_id: u64,
    pub database_pool: DatabasePool,
    pub audio_folder_path: String,
}
