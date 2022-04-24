use actix::Addr;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};
use teloxide::prelude::*;

use crate::{discord::actor::DiscordActor, lock::lock_actor::SoundLockActor};

pub type DatabasePool = Pool<ConnectionManager<SqliteConnection>>;

pub struct AppState {
    pub app_name: String,
    pub discord_actor_addr: Addr<DiscordActor>,
    pub sound_lock_actor_addr: Addr<SoundLockActor>,
    pub discord_guild_id: u64,
    pub database_pool: DatabasePool,
    pub audio_folder_path: String,
    pub telegram_bot: AutoSend<Bot>,
    pub telegram_chat_id: String,
}
