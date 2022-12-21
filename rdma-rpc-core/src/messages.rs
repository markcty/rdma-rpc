use core::{cmp::Ordering, fmt::Display};

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use KRdmaKit::services_user::ibv_gid_wrapper;

/// Packet is the base element transmitted on the rdma network
#[derive(Serialize, Deserialize, Debug, Clone, Eq)]
pub(crate) struct Packet {
    ack_num: u64,
    seq_num: u64,
    is_ack: bool,
    total_num: u64, // number of packets in current req/resp
    session_id: u64,
    data: Vec<u8>,
}

impl Packet {
    pub(crate) fn new_ack(ack: u64, syn: u64, session_id: u64) -> Packet {
        Packet {
            ack_num: ack,
            seq_num: syn,
            is_ack: true,
            total_num: 0,
            session_id,
            data: Vec::new(),
        }
    }

    pub(crate) fn new(
        ack: u64,
        syn: u64,
        session_id: u64,
        data: Vec<u8>,
        total_num: u64,
    ) -> Packet {
        Self {
            ack_num: ack,
            seq_num: syn,
            total_num,
            is_ack: false,
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

    pub(crate) fn total_num(&self) -> u64 {
        self.total_num
    }

    pub(crate) fn data(&self) -> &[u8] {
        self.data.as_slice()
    }

    pub(crate) fn is_ack(&self) -> bool {
        self.is_ack
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
