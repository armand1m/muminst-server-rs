use actix::prelude::*;

#[derive(Clone, Message)]
#[rtype(result = "()")]
pub struct LockSound {}

#[derive(Clone, Message)]
#[rtype(result = "()")]
pub struct UnlockSound {}
