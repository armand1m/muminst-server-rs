use std::{path::PathBuf, sync::Arc};

use actix::prelude::*;
use serenity::model::prelude::GuildId;
use songbird::{
    driver::Bitrate,
    input::{self, cached::Compressed},
    Songbird,
};

pub struct DiscordActor {
    pub discord_guild_id: u64,
    pub songbird: Arc<Songbird>,
}

/// Define message
#[derive(Message)]
#[rtype(result = "()")]
pub struct PlayAudio {
    pub audio_path: PathBuf,
}

impl Actor for DiscordActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        println!("DiscordActor is alive");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        println!("DiscordActor is stopped");
    }
}

impl Handler<PlayAudio> for DiscordActor {
    type Result = ();

    fn handle(&mut self, msg: PlayAudio, ctx: &mut Self::Context) -> Self::Result {
        let audio_path = msg.audio_path;
        let guild_id: GuildId = self.discord_guild_id.into();
        let manager = self.songbird.clone();

        async move {
            if let Some(handler_lock) = manager.get(guild_id) {
                let bitrate = Bitrate::BitsPerSecond(48_000);
                let audio_source = input::ffmpeg(&audio_path).await.expect("Link may be dead.");
                let sound_src = Compressed::new(audio_source, bitrate)
                    .expect("ffmpeg parameters to be properly defined");

                let mut handler = handler_lock.lock().await;
                handler.play_source(sound_src.into());
            }
        }
        .into_actor(self)
        .wait(ctx);
    }
}
