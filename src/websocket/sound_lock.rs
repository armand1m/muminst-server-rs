use crate::app_state::AppState;
use crate::lock::lock_actor::SoundLockActor;
use crate::lock::messages::{GetLockStatus, GetLockStatusResponse, WsLockSound, WsUnlockSound};
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

#[derive(Clone)]
struct SoundLockWsActor {
    sound_lock_actor_addr: Addr<SoundLockActor>,
    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    heartbeat_ts: Instant,
}

impl SoundLockWsActor {
    pub fn new(sound_lock_actor_addr: Addr<SoundLockActor>) -> Self {
        Self {
            sound_lock_actor_addr,
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

impl Actor for SoundLockWsActor {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start. We start the heartbeat process here.
    fn started(&mut self, ctx: &mut Self::Context) {
        info!("Starting one SoundlockWsActor");
        self.heartbeat(ctx);
        self.subscribe_system_async::<WsLockSound>(ctx);
        self.subscribe_system_async::<WsUnlockSound>(ctx);
        async move {
            let result = self.sound_lock_actor_addr.send(GetLockStatus {}).await?;
            match result {
                Some(status) => {
                    if status.is_locked {
                        ctx.text("{ \"isLocked\": true }")
                    } else {
                        ctx.text("{ \"isLocked\": false }")
                    }
                }
                None => ctx.text("{ \"isLocked\": false }"),
            }
            Ok(result)
        }
        .into_actor(self)
        // TODO: Solve this error or find another way to run this future
        .spawn(ctx);
    }
}

impl Handler<GetLockStatusResponse> for SoundLockWsActor {
    type Result = ();

    fn handle(&mut self, msg: GetLockStatusResponse, ctx: &mut Self::Context) -> Self::Result {
        info!(" Receiving GetLockStatusResponse {:?}", msg);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for SoundLockWsActor {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        // process websocket messages
        // info!("stream_handler message: {:?}", msg);

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
            Ok(ws::Message::Pong(_msg)) => {
                // info!("pong message: {:?}", msg);
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

impl Handler<WsLockSound> for SoundLockWsActor {
    type Result = ();

    fn handle(&mut self, _msg: WsLockSound, ctx: &mut Self::Context) -> Self::Result {
        info!("sending lock to client");
        ctx.text("{ \"isLocked\": true }")
    }
}

impl Handler<WsUnlockSound> for SoundLockWsActor {
    type Result = ();

    fn handle(&mut self, _msg: WsUnlockSound, ctx: &mut Self::Context) -> Self::Result {
        info!("sending unlock to client");
        ctx.text("{ \"isLocked\": false }")
    }
}

pub async fn sound_lock_handler(
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    info!("Receive /ws request");
    let app_data = req.app_data::<AppState>().unwrap();
    let actor = SoundLockWsActor::new(app_data.sound_lock_actor_addr.clone());
    ws::start(actor, &req, stream)
}
