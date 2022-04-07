use actix::{Actor, Context, Handler};
use actix_broker::{Broker, SystemBroker};

use super::messages::{GetLockStatus, Lock, LockStatus, Unlock, WsLockSound, WsUnlockSound};

pub struct SoundLockActor {
    status: LockStatus,
}

impl Handler<Lock> for SoundLockActor {
    type Result = ();

    fn handle(&mut self, msg: Lock, _ctx: &mut Context<Self>) -> Self::Result {
        Broker::<SystemBroker>::issue_async(WsLockSound {});
        self.status = LockStatus {
            sound: Some(msg.sound.clone()),
            is_locked: true,
        }
    }
}

impl Handler<Unlock> for SoundLockActor {
    type Result = ();

    fn handle(&mut self, _msg: Unlock, _ctx: &mut Context<Self>) -> Self::Result {
        Broker::<SystemBroker>::issue_async(WsUnlockSound {});
        self.status = LockStatus {
            sound: None,
            is_locked: false,
        }
    }
}

impl Handler<GetLockStatus> for SoundLockActor {
    type Result = Option<LockStatus>;

    fn handle(&mut self, _msg: GetLockStatus, _ctx: &mut Context<Self>) -> Self::Result {
        Some(self.status.clone())
    }
}

impl SoundLockActor {
    pub fn new() -> Self {
        Self {
            status: LockStatus {
                sound: None,
                is_locked: false,
            },
        }
    }
}

impl Actor for SoundLockActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {}
}
