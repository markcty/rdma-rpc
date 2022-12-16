use core::fmt::Display;

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use KRdmaKit::services_user::ibv_gid_wrapper;

/// Packet is the base element transmitted on the rdma network
#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct Packet {
    ack_num: u64,
    seq_num: u64,
    pub fin: u8,
    pub syn: u8,
    pub ack: u8,
    session_id: u64,
    data: Vec<u8>, // typically: this should be Vec<u8>
}

impl Packet {
    pub(crate) fn new_empty(ack: u64, syn: u64, session_id: u64) -> Packet {
        Packet {
            ack_num: ack,
            seq_num: syn,
            fin: 0,
            syn: 0,
            ack: 0,
            session_id,
            data: Vec::new(),
        }
    }
    pub(crate) fn new_fin(ack: u64, syn: u64, session_id: u64) -> Packet {
        Packet {
            ack_num: ack,
            seq_num: syn,
            fin: 1,
            syn: 0,
            ack: 0,
            session_id,
            data: Vec::new(),
        }
    }

    pub(crate) fn new(ack: u64, syn: u64, session_id: u64, data: Vec<u8>) -> Packet {
        Self {
            ack_num: ack,
            seq_num: syn,
            fin: 0,
            syn: 0,
            ack: 0,
            session_id,
            data,
        }
    }

    pub(crate) fn session_id(&self) -> u64 {
        self.session_id
    }

    pub(crate) fn ack(&self) -> u64 {
        self.ack_num
    }

    pub(crate) fn seq(&self) -> u64 {
        self.seq_num
    }
    #[allow(unused)]
    pub(crate) fn raw_data(&self) -> Vec<u8> {
        self.data.clone()
    }
    pub(crate) fn data(&self) -> &[u8] {
        self.data.as_slice()
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
