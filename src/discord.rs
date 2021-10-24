pub mod commands;

use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::gateway::Ready,
};

pub struct DiscordHandler;

#[async_trait]
impl EventHandler for DiscordHandler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("Bot connected as \"{}\".", ready.user.name);
    }
}
