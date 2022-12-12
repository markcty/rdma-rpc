use alloc::{string::ToString, sync::Arc};
use serde::Serialize;
use KRdmaKit::{
    context::Context, log::error, services_user::ibv_gid_wrapper, DatagramEndpoint, MemoryRegion,
    QueuePair,
};

use crate::{
    error::Error,
    messages::{Packet, QPInfo},
};

const BUF_SIZE: u64 = 1024;

pub struct Transport {
    qp: Arc<QueuePair>,
    endpoint: DatagramEndpoint,
    mr: Arc<MemoryRegion>,
}

impl Transport {
    /// Connect to the server and
    pub fn new(
        qp: Arc<QueuePair>,
        qp_info: QPInfo,
        ctx: Arc<Context>,
        mr: Arc<MemoryRegion>,
    ) -> Result<Self, Error> {
        // create the remote endpoint
        let endpoint = DatagramEndpoint::new(
            &ctx,
            1,
            qp_info.lid,
            qp_info.gid.into(),
            qp_info.qp_num,
            qp_info.qkey,
        )
        .expect("UD endpoint creation fails");
        Ok(Self { qp, endpoint, mr })
    }

    pub(crate) fn send<T: Serialize>(&self, session_id: u64, data: T) -> Result<(), Error> {
        let packet = Packet::new(session_id, data);

        // serialize arg
        let buffer: &mut [u8] = unsafe {
            alloc::slice::from_raw_parts_mut(self.mr.get_virt_addr() as _, BUF_SIZE as usize)
        };
        bincode::serialize_into(buffer, &packet).map_err(|err| {
            error!("failed to serialize rpc args, {err}");
            Error::EncodeArgs
        })?;
        let size = bincode::serialized_size(&packet).map_err(|err| {
            error!("failed to serialize rpc args, {err}");
            Error::EncodeArgs
        })?;

        // post send
        self.qp
            .post_datagram(&self.endpoint, &self.mr, 0..size, 1, true)
            .map_err(|err| {
                error!("failed to post datagram: {err}");
                Error::Internal(err.to_string())
            })?;

        Ok(())
    }

    /// Get self qp info
    pub fn self_qp_info(&self) -> QPInfo {
        QPInfo {
            lid: self.qp.lid().unwrap(),
            gid: ibv_gid_wrapper::from(self.qp.gid().unwrap()),
            qp_num: self.qp.qp_num(),
            qkey: self.qp.qkey(),
        }
    }
}
