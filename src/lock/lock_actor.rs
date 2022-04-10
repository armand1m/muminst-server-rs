use actix::{Actor, Context, Handler};
use actix_broker::{BrokerIssue, BrokerSubscribe};
use log::info;

use super::messages::{GetLockStatus, Lock, LockStatus, Unlock, WsLockSound, WsUnlockSound};

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
        info!("Handling LOCK from lock_actor");
        self.status = LockStatus {
            sound: Some(msg.sound.clone()),
            is_locked: true,
        };
        self.issue_system_async(WsLockSound {});
    }
}

impl Handler<Unlock> for SoundLockActor {
    type Result = ();

    fn handle(&mut self, _msg: Unlock, _ctx: &mut Context<Self>) -> Self::Result {
        self.status = LockStatus::new();
        self.issue_system_async(WsUnlockSound {});
    }
}

impl Handler<GetLockStatus> for SoundLockActor {
    type Result = Option<LockStatus>;

    fn handle(&mut self, _msg: GetLockStatus, _ctx: &mut Context<Self>) -> Self::Result {
        Some(self.status.clone())
    }
}
