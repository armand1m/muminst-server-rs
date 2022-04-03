use std::{path::PathBuf, sync::Arc};

use actix::prelude::*;
use log::info;
use serenity::{async_trait, model::prelude::GuildId};
use songbird::{
    driver::Bitrate,
    input::{self, cached::Compressed},
    Event, EventContext, EventHandler as VoiceEventHandler, Songbird, TrackEvent,
};

struct SongEndNotifier {}

#[async_trait]
impl VoiceEventHandler for SongEndNotifier {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        info!("Song ended");

        // TODO: send message to unlock actor here
        None
    }
}

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
        info!("DiscordActor is alive");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!("DiscordActor is stopped");
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

                // TODO: send message to lock actor here
                let mut handler = handler_lock.lock().await;
                let song = handler.play_source(sound_src.into());

                // This shows how to fire an event once an audio track completes,
                // either due to hitting the end of the bytestream or stopped by user code.
                //
                // TODO: send message to unlock actor here
                let _ = song.add_event(Event::Track(TrackEvent::End), SongEndNotifier {});
            }
        }
        .into_actor(self)
        .wait(ctx);
    }
}
