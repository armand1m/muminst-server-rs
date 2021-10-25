pub mod commands;

use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::gateway::Ready,
};
pub struct DiscordHandler;

#[async_trait]
impl EventHandler for DiscordHandler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("Bot connected as \"{}\".", ready.user.name);
        {
            let ctx_mutex = &*crate::DISCORD_CTX;
            *ctx_mutex.lock().unwrap() = Some(ctx);
        }
    }
}
