use crate::lock::messages::{GetLockStatus, Lock, LockStatus, Unlock, WsLockSound, WsUnlockSound};
use actix::{Actor, Context, Handler};
use actix_broker::{BrokerIssue, BrokerSubscribe};
use log::{debug, info};

#[derive(Clone)]
pub struct SoundLockActor {
    status: LockStatus,
}

impl SoundLockActor {
    pub fn new() -> Self {
        Self {
            status: LockStatus::new(),
        }
    }
}

impl Actor for SoundLockActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.subscribe_system_async::<Lock>(ctx);
        self.subscribe_system_async::<Unlock>(ctx);
    }
}

impl Handler<Lock> for SoundLockActor {
    type Result = ();

    fn handle(&mut self, msg: Lock, _ctx: &mut Context<Self>) -> Self::Result {
        info!("handling lock with sound '{}'", msg.sound.name);
        self.status = LockStatus {
            sound: Some(msg.sound),
            is_locked: true,
        };
        debug!("set status to {:?}", self.status);
        self.issue_system_async(WsLockSound {});
    }
}

impl Handler<Unlock> for SoundLockActor {
    type Result = ();

    fn handle(&mut self, _msg: Unlock, _ctx: &mut Context<Self>) -> Self::Result {
        info!("handling unlock");
        self.status = LockStatus::new();
        debug!("set status to {:?}", self.status);
        self.issue_system_async(WsUnlockSound {});
    }
}

impl Handler<GetLockStatus> for SoundLockActor {
    type Result = Option<LockStatus>;

    fn handle(&mut self, _msg: GetLockStatus, _ctx: &mut Context<Self>) -> Self::Result {
        let status = self.status.clone();
        debug!("replying to get lock status with {:?}", status);
        Some(status)
    }
}
