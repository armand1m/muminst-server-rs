use actix::Message;

use crate::models::Sound;

#[derive(Debug, Clone)]
pub struct LockStatus {
    pub is_locked: bool,
    pub sound: Option<Sound>,
}

impl LockStatus {
    pub fn new() -> LockStatus {
        LockStatus {
            is_locked: false,
            sound: None,
        }
    }
}

#[derive(Clone, Message)]
#[rtype(result = "()")]
pub struct WsLockSound {}

#[derive(Clone, Message)]
#[rtype(result = "()")]
pub struct WsUnlockSound {}

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct Lock {
    pub sound: Sound,
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct Unlock;

#[derive(Message, Clone, Debug)]
#[rtype(result = "Option<LockStatus>")]
pub struct GetLockStatus;

#[derive(Message, Clone, Debug)]
#[rtype(result = "()")]
pub struct GetLockStatusResponse {
    pub status: LockStatus,
}
