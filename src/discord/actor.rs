use std::{path::PathBuf, sync::Arc};

use crate::lock::messages::LockSound;
use actix::prelude::*;
use actix_broker::{Broker, SystemBroker};
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
        // TODO: broken. panics because cannot find the tokio runtime
        // for some reason.
        //
        // I assume this is because the event is being triggered from
        // within an async trait, but I'm not sure how to fix it.
        //
        // I'm capable of gettin the current handler with
        // tokio::runtime::Handler::current() but when
        // trying to use it to spawn an async task, it
        // fails in the same way.
        //
        // Creating a new system causes the following to happen:
        //
        // thread 'tokio-runtime-worker' panicked at 'Cannot start a runtime from within a runtime. This happens because a function (like `block_on`) attempted to block the current thread while the thread is being used to drive asynchronous tasks.', /Users/amagalhaes/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.17.0/src/runtime/enter.rs:39:9
        //
        // Running `System::is_registered()` here returns `false`, while
        // in the Actor it returns `true`
        //
        // Broker::<SystemBroker>::issue_async(UnlockSound {});

        info!("has system registered: {:?}", System::is_registered());

        None
    }
}

pub struct DiscordActor {
    pub discord_guild_id: u64,
    pub songbird: Arc<Songbird>,
}

/// Define message
#[derive(Clone, Message)]
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

                let mut handler = handler_lock.lock().await;

                info!("has system registered: {:?}", System::is_registered());

                Broker::<SystemBroker>::issue_async(LockSound {});
                let song = handler.play_source(sound_src.into());
                let _ = song.add_event(Event::Track(TrackEvent::End), SongEndNotifier {});
            }
        }
        .into_actor(self)
        .wait(ctx);
    }
}
