use actix::Message;

use crate::models::Sound;

#[derive(Clone)]
pub struct LockStatus {
    pub is_locked: bool,
    pub sound: Option<Sound>,
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

#[derive(Message, Clone)]
#[rtype(result = "Option<LockStatus>")]
pub struct GetLockStatus;
