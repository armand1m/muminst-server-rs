use std::sync::{Arc, Mutex};

use serenity::client::Context;

pub struct AppState {
    pub app_name: String,
    pub discord_ctx: Arc<Mutex<Option<Context>>>,
}
