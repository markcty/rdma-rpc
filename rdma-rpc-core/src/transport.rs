use alloc::{string::ToString, sync::Arc};
use serde::{de::DeserializeOwned, Serialize};
use KRdmaKit::{
    context::Context,
    log::{error, info},
    services_user::ibv_gid_wrapper,
    DatagramEndpoint, MemoryRegion, QueuePair, QueuePairBuilder,
};

use crate::{
    error::Error,
    messages::{Packet, QPInfo},
};

// for the MR, its layout is:
// |0    ... 4096 | // send buffer
// |4096 ... 8192 | // receive buffer
pub const BUF_SIZE: u64 = 4096; // 4KB
const MR_SIZE: u64 = BUF_SIZE * 2; // 8KB
const UD_DATA_OFFSET: usize = 40; // For a UD message, the first 40 bytes are reserved for GRH

pub struct Transport {
    qp: Arc<QueuePair>,
    endpoint: DatagramEndpoint,
    mr: MemoryRegion,
}

impl Transport {
    /// Connect to the server and
    pub fn new(context: Arc<Context>, qp_info: QPInfo, port: u8) -> Result<Self, Error> {
        // create a qp and the remote endpoint
        let qp = {
            let builder = QueuePairBuilder::new(&context);
            let qp = builder
                .build_ud()
                .expect("failed to build UD QP")
                .bring_up_ud()
                .expect("failed to bring up UD QP");
            info!("QP status: {:?}", qp.status());
            qp
        };
        info!("QP num: {:?}, qkey: {:?}", qp.qp_num(), qp.qkey());
        let endpoint = DatagramEndpoint::new(
            &context,
            port,
            qp_info.lid,
            qp_info.gid.into(),
            qp_info.qp_num,
            qp_info.qkey,
        )
        .map_err(|_| Error::Internal("UD endpoint creation fails".to_string()))?;

        // create mr
        let mr = MemoryRegion::new(Arc::clone(&context), MR_SIZE as usize)
            .expect("failed to allocate MR");

        // init post recv
        qp.post_recv(&mr, BUF_SIZE..MR_SIZE, 1)
            .map_err(|err| Error::Internal(alloc::format!("internal error: {err}")))?;

        Ok(Self { qp, endpoint, mr })
    }

    pub fn new_with_qp(
        qp: Arc<QueuePair>,
        context: Arc<Context>,
        qp_info: QPInfo,
        port: u8,
    ) -> Result<Self, Error> {
        let endpoint = DatagramEndpoint::new(
            &context,
            port,
            qp_info.lid,
            qp_info.gid.into(),
            qp_info.qp_num,
            qp_info.qkey,
        )
        .map_err(|_| Error::Internal("UD endpoint creation fails".to_string()))?;

        // create mr
        let mr = MemoryRegion::new(Arc::clone(&context), MR_SIZE as usize)
            .expect("failed to allocate MR");

        // init post recv
        qp.post_recv(&mr, BUF_SIZE..MR_SIZE, 1)
            .map_err(|err| Error::Internal(alloc::format!("internal error: {err}")))?;

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

    pub(crate) fn recv<R: DeserializeOwned>(&self) -> Result<Packet<R>, Error> {
        // poll recv cq
        let mut wcs = [Default::default()];
        let res = loop {
            let res = self
                .qp
                .poll_recv_cq(&mut wcs)
                .expect("failed to poll recv CQ");
            if !res.is_empty() {
                break res;
            }
        };

        // dserialize arg
        assert!(res.len() == 1);
        let msg_sz = res[0].byte_len as usize - UD_DATA_OFFSET;
        let msg: Packet<R> = bincode::deserialize(unsafe {
            alloc::slice::from_raw_parts(
                (self.mr.get_virt_addr() as usize + BUF_SIZE as usize + UD_DATA_OFFSET) as *mut u8,
                msg_sz,
            )
        })
        .map_err(|_| {
            self.qp
                .post_recv(&self.mr, BUF_SIZE..MR_SIZE, 1)
                .expect("failed to post recv");
            Error::DecodeArgs
        })?;

        // post recv
        self.qp
            .post_recv(&self.mr, BUF_SIZE..MR_SIZE, 1)
            .map_err(|err| Error::Internal(alloc::format!("internal error: {err}")))?;

        Ok(msg)
    }
}
