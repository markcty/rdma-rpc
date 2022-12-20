use core::fmt::Display;

use serde::{Deserialize, Serialize};
use KRdmaKit::services_user::ibv_gid_wrapper;

/// Packet is the base element transmitted on the rdma network
#[derive(Serialize, Deserialize)]
pub(crate) struct Packet<T> {
    // TODO: add header like ack, syn
    session_id: u64,
    data: T, // typically: this should be Vec<u8>
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
    pub gid: ibv_gid_wrapper,
    pub qp_num: u32,
    pub qkey: u32,
}

impl Display for QPInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}
