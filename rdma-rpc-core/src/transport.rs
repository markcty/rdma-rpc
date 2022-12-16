use alloc::{format, string::ToString, sync::Arc};
use tracing::{error, info};
use KRdmaKit::{
    context::Context,
    services_user::{self},
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
    pub mr: MemoryRegion,
}

impl Transport {
    /// Connect to the server
    pub fn new(context: Arc<Context>, qp_info: QPInfo, port: u8) -> Result<Self, Error> {
        // create a qp and the remote endpoint
        let qp = QueuePairBuilder::new(&context)
            .build_ud()
            .map_err(|err| Error::Internal(format!("failed to build ud, {err}")))?
            .bring_up_ud()
            .map_err(|err| Error::Internal(format!("failed to bring up ud, {err}")))?;

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
            .map_err(|err| Error::Internal(format!("failed to allocate MR, {err}")))?;

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
            .map_err(|err| Error::Internal(format!("failed to allocate MR, {err}")))?;

        // init post recv
        qp.post_recv(&mr, BUF_SIZE..MR_SIZE, 1)
            .map_err(|err| Error::Internal(alloc::format!("internal error: {err}")))?;

        Ok(Self { qp, endpoint, mr })
    }
    pub(crate) fn send(&self, packet: Packet) -> Result<(), Error> {
        // serialize arg
        let buffer: &mut [u8] = unsafe {
            alloc::slice::from_raw_parts_mut(self.mr.get_virt_addr() as _, BUF_SIZE as usize)
        };
        bincode::serialize_into(buffer, &packet)?;
        let size = bincode::serialized_size(&packet)?;

        // post send
        self.qp
            .post_datagram(&self.endpoint, &self.mr, 0..size, 1, true)
            .map_err(|err| {
                error!("failed to post datagram: {err}");
                Error::Internal(err.to_string())
            })?;
        info!(
            "transport send packet to remote qpn {:?}",
            self.endpoint.qpn()
        );

        Ok(())
    }
    pub(crate) fn send_u8(
        &self,
        mr: &MemoryRegion,
        start_pos: u64,
        end_pos: u64,
    ) -> Result<(), Error> {
        // post send
        self.qp
            .post_datagram(&self.endpoint, mr, start_pos..end_pos, 1, true)
            .map_err(|err| {
                error!("failed to post datagram: {err}");
                Error::Internal(err.to_string())
            })?;
        info!(
            "transport send packet to remote qpn {:?}",
            self.endpoint.qpn()
        );

        Ok(())
    }

    pub(crate) fn recv(&self) -> Result<Packet, Error> {
        // poll recv cq
        let mut wcs = [Default::default()];
        let res = loop {
            let res = self
                .qp
                .poll_recv_cq(&mut wcs)
                .map_err(|err| Error::Internal(format!("failed to poll cq, {err}")))?;
            if !res.is_empty() {
                break res;
            }
        };
        info!("transport recv packet");

        // post recv
        self.qp
            .post_recv(&self.mr, BUF_SIZE..MR_SIZE, 1)
            .map_err(|err| Error::Internal(alloc::format!("internal error: {err}")))?;

        // deserialize arg
        assert!(res.len() == 1);
        let msg_sz = res[0].byte_len as usize - UD_DATA_OFFSET;
        let msg = bincode::deserialize(unsafe {
            alloc::slice::from_raw_parts(
                (self.mr.get_virt_addr() as usize + BUF_SIZE as usize + UD_DATA_OFFSET) as *mut u8,
                msg_sz,
            )
        })?;

        Ok(msg)
    }

    pub(crate) fn recv_fa<R: DeserializeOwned>(&self) -> Result<Packet<R>, Error> {
        // poll recv cq
        let mut wcs = [Default::default()];
        let res = {
            let res = self
                .qp
                .poll_recv_cq(&mut wcs)
                .map_err(|err| Error::Internal(format!("failed to poll cq, {err}")))?;
            if !res.is_empty() {
                res
            } else {
                return Err(Error::Receive);
            }
        };
        info!("transport recv packet");

        // post recv
        self.qp
            .post_recv(&self.mr, BUF_SIZE..MR_SIZE, 1)
            .map_err(|err| Error::Internal(alloc::format!("internal error: {err}")))?;

        // deserialize arg
        assert!(res.len() == 1);
        let msg_sz = res[0].byte_len as usize - UD_DATA_OFFSET;
        let msg: Packet<R> = bincode::deserialize(unsafe {
            alloc::slice::from_raw_parts(
                (self.mr.get_virt_addr() as usize + BUF_SIZE as usize + UD_DATA_OFFSET) as *mut u8,
                msg_sz,
            )
        })?;

        Ok(msg)
    }

    pub(crate) fn try_recv(&self) -> Result<Option<Packet>, Error> {
        // poll recv cq
        let mut wcs = [Default::default()];

        let res = self
            .qp
            .poll_recv_cq(&mut wcs)
            .map_err(|err| Error::Internal(format!("failed to poll cq, {err}")))?;
        if res.is_empty() {
            return Ok(None);
        }

        info!("transport try recv succeeded");

        // post recv
        self.qp
            .post_recv(&self.mr, BUF_SIZE..MR_SIZE, 1)
            .map_err(|err| Error::Internal(alloc::format!("internal error: {err}")))?;

        // deserialize arg
        assert!(res.len() == 1);
        let msg_sz = res[0].byte_len as usize - UD_DATA_OFFSET;
        let msg = bincode::deserialize(unsafe {
            alloc::slice::from_raw_parts(
                (self.mr.get_virt_addr() as usize + BUF_SIZE as usize + UD_DATA_OFFSET) as *mut u8,
                msg_sz,
            )
        })?;

        Ok(Some(msg))
    }

    pub fn qp_info(&self) -> QPInfo {
        QPInfo {
            lid: self.qp.lid().unwrap(),
            gid: services_user::ibv_gid_wrapper::from(self.qp.gid().unwrap()),
            qp_num: self.qp.qp_num(),
            qkey: self.qp.qkey(),
        }
    }
}
