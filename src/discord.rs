pub mod commands;

use std::sync::{Arc, Mutex};

use serenity::{async_trait, client::{Context, EventHandler}, model::gateway::Ready, prelude::TypeMapKey};

pub struct ContextStore;

impl TypeMapKey for ContextStore {
    type Value = Arc<Mutex<Option<Context>>>;
}

pub struct DiscordHandler {}

#[async_trait]
impl EventHandler for DiscordHandler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("Bot connected as \"{}\".", ready.user.name);

        {
            let mut data = ctx.data.write().await;
            data.insert::<ContextStore>(Arc::new(Mutex::new(Some(ctx.clone()))));
        }
    }
}
