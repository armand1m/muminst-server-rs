use crate::lock::messages::{LockSound, UnlockSound};
use actix::prelude::*;
use actix::{Actor, StreamHandler};
use actix_broker::BrokerSubscribe;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use log::info;
use std::time::{Duration, Instant};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

struct SoundLockWSHandler {
    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    heartbeat_ts: Instant,
}

impl SoundLockWSHandler {
    pub fn new() -> Self {
        Self {
            heartbeat_ts: Instant::now(),
        }
    }

    /// helper method that sends ping to client every second.
    /// also this method checks heartbeats from client
    fn heartbeat(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.heartbeat_ts) > CLIENT_TIMEOUT {
                info!("Websocket client heartbeat timed out. Disconnecting.");
                ctx.stop();
                return;
            }

            ctx.ping(b"");
        });
    }
}

impl Actor for SoundLockWSHandler {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start. We start the heartbeat process here.
    fn started(&mut self, ctx: &mut Self::Context) {
        self.heartbeat(ctx);
        self.subscribe_system_async::<LockSound>(ctx);
        self.subscribe_system_async::<UnlockSound>(ctx);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for SoundLockWSHandler {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        // process websocket messages
        info!("stream_handler message: {:?}", msg);

        match msg {
            Ok(ws::Message::Ping(msg)) => {
                ctx.pong(&msg);
            }
            Ok(ws::Message::Text(msg)) => {
                // This is some sort of a hack and should not be needed,
                // since the ws spec has it's own ping implementation which
                // actix already implements
                if msg == "{\"type\":\"PING\"}" {
                    ctx.pong(b"")
                }
            }
            Ok(ws::Message::Pong(msg)) => {
                info!("pong message: {:?}", msg);
                self.heartbeat_ts = Instant::now();
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

impl Handler<LockSound> for SoundLockWSHandler {
    type Result = ();

    fn handle(&mut self, _msg: LockSound, ctx: &mut Self::Context) -> Self::Result {
        info!("sending lock to clients");
        ctx.text("{ \"isLocked\": true }")
    }
}

impl Handler<UnlockSound> for SoundLockWSHandler {
    type Result = ();

    fn handle(&mut self, _msg: UnlockSound, ctx: &mut Self::Context) -> Self::Result {
        info!("sending unlock to clients");
        ctx.text("{ \"isLocked\": false }")
    }
}

pub async fn sound_lock_handler(
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    let handler = SoundLockWSHandler::new();
    ws::start(handler, &req, stream)
}
