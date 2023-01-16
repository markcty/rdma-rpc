use alloc::vec::Vec;
use core::{cmp::Ordering, fmt::Display};

use serde::{Deserialize, Serialize};
use KRdmaKit::services_user::ibv_gid_wrapper;

/// Packet is the base element transmitted on the rdma network
#[derive(Serialize, Deserialize, Debug, Clone, Eq)]
pub(crate) struct Packet {
    is_ack: bool,
    ack_num: u64,
    seq_num: u64,
    session_id: u64,
    data: Vec<u8>,
}

impl Packet {
    pub(crate) fn new_ack(ack_num: u64, session_id: u64) -> Packet {
        Packet {
            is_ack: true,
            ack_num,
            seq_num: 0,
            session_id,
            data: Vec::new(),
        }
    }

    pub(crate) fn new(seq_num: u64, session_id: u64, data: Vec<u8>) -> Packet {
        Self {
            is_ack: false,
            ack_num: 0,
            seq_num,
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

    pub(crate) fn is_ack(&self) -> bool {
        self.is_ack
    }

    pub(crate) fn into_data(self) -> Vec<u8> {
        self.data
    }
}

impl Ord for Packet {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.seq_num.cmp(&other.seq_num)
    }
}

impl PartialOrd for Packet {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Packet {
    fn eq(&self, other: &Self) -> bool {
        self.seq_num == other.seq_num
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
