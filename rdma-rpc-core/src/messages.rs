use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct Packet<T> {
    // TODO: add header like ack, syn
    session_id: u64,
    data: T,
}

impl<T> Packet<T> {
    pub(crate) fn new(session_id: u64, data: T) -> Packet<T> {
        Self { session_id, data }
    }

    pub(crate) fn into_inner(self) -> T {
        self.data
    }

    pub(crate) fn session_id(&self) -> u64 {
        self.session_id
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct QPInfo {
    pub lid: u32,
    pub gid: u128,
    pub qp_num: u32,
    pub qkey: u32,
}
