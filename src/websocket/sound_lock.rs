use actix::prelude::*;
use actix::{Actor, StreamHandler};
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
    heartbeat: Instant,
}

impl SoundLockWSHandler {
    pub fn new() -> Self {
        Self {
            heartbeat: Instant::now(),
        }
    }

    /// helper method that sends ping to client every second.
    /// also this method checks heartbeats from client
    fn heartbeat(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.heartbeat) > CLIENT_TIMEOUT {
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
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for SoundLockWSHandler {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        // process websocket messages
        info!("{:?}", msg);

        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.heartbeat = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.heartbeat = Instant::now();
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct LockSound {}

impl Handler<LockSound> for SoundLockWSHandler {
    type Result = ();

    fn handle(&mut self, _msg: LockSound, ctx: &mut Self::Context) -> Self::Result {
        info!("sending lock to clients");
        ctx.text("{ \"isLocked\": true }")
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct UnlockSound {}

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
    ws::start(SoundLockWSHandler::new(), &req, stream)
}
