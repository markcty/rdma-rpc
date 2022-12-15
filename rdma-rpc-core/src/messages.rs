use core::fmt::Display;

use serde::{Deserialize, Serialize};
use KRdmaKit::services_user::ibv_gid_wrapper;

/// Packet is the base element transmitted on the rdma network
#[derive(Serialize, Deserialize)]
pub(crate) struct Packet<T> {
    // TODO: add header like ack, syn
    checksum: u32,
    seq_num: u32,
    ack_num: u32,
    session_id: u64,
    data: T, // typically: this should be Vec<u8>
}

impl<T> Packet<T> {
    pub(crate) fn new(seq_num: u32, ack_num: u32, session_id: u64, data: T) -> Packet<T> {
        let mut ret_Pck = Self {
            checksum: 0u32,
            seq_num,
            ack_num,
            session_id,
            data,
        };
        ret_Pck.set_checksum();
        ret_Pck
    }

    pub(crate) fn into_inner(self) -> T {
        self.data
    }

    pub(crate) fn session_id(&self) -> u64 {
        self.session_id
    }
    pub(crate) fn ack_num(&self) -> u32 {
        self.ack_num
    }
    pub(crate) fn seq_num(&self) -> u32 {
        self.seq_num
    }
    fn set_checksum(&mut self) {
        self.checksum = self.gen_checksum()
    }
    pub(crate) fn check_checksum(&self) -> bool {
        self.gen_checksum() == self.checksum
    }
    fn gen_checksum(&self) -> u32 {
        // let mut check_sum = 0u32;
        // bincode::serialize_into(buffer, &self)?;
        // let size = bincode::serialized_size(&packet);
        1u32
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
        write!(f, "{:?}", self)
    }
}
