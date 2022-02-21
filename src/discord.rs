pub mod commands;
use log::info;

use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{event::ResumedEvent, gateway::Ready},
};
pub struct DiscordHandler;

#[async_trait]
impl EventHandler for DiscordHandler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("Discord Client connected as \"{}\".", ready.user.name);
        {
            let ctx_mutex = &*crate::DISCORD_CTX;
            *ctx_mutex.lock().unwrap() = Some(ctx);
            info!("Mutex updated with reference to Discord Context");
        }
    }

    async fn resume(&self, _ctx: Context, _: ResumedEvent) {
        info!("Discord Client connection was resumed.");
    }
}
