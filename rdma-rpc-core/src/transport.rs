use alloc::{collections::BTreeMap, format, string::ToString, sync::Arc, vec::Vec};
use serde::{de::DeserializeOwned, Serialize};
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

const BUF_SIZE: u64 = 4096; // 4KB
const UD_DATA_OFFSET: usize = 40; // for a UD message, the first 40 bytes are reserved for GRH
const POOL_SIZE: u8 = 8; // how many mrs are in a mr pool

struct MemoryRegionWrapper {
    mr: Arc<MemoryRegion>,
    used: bool, // whether it's being used now or free
}

struct MrPool {
    mrs: BTreeMap<u64, MemoryRegionWrapper>,
}

impl MrPool {
    fn new(context: Arc<Context>, num: u8) -> Result<Self, Error> {
        let mut mrs = BTreeMap::new();
        for i in 0..num {
            let mr = Arc::new(
                MemoryRegion::new(Arc::clone(&context), BUF_SIZE as usize)
                    .map_err(|err| Error::Internal(format!("failed to allocate MR, {err}")))?,
            );
            mrs.insert(i as u64, MemoryRegionWrapper { mr, used: false });
        }
        Ok(Self { mrs })
    }

    fn get_free_mr(&mut self) -> Option<(u64, Arc<MemoryRegion>)> {
        self.mrs.iter_mut().find_map(|(id, mr)| {
            (!mr.used).then(|| {
                mr.used = true;
                (*id, Arc::clone(&mr.mr))
            })
        })
    }

    fn mark_mr_free(&mut self, id: u64) -> Result<(), Error> {
        match self.mrs.get_mut(&id) {
            Some(mr) => {
                mr.used = false;
                Ok(())
            }
            None => Err(Error::Internal(format!("mr of id {id} doesn't exist"))),
        }
    }

    fn get_mr_with_id(&self, id: u64) -> Result<Arc<MemoryRegion>, Error> {
        match self.mrs.get(&id) {
            Some(mr) => Ok(Arc::clone(&mr.mr)),
            None => Err(Error::Internal(format!("mr of id {id} doesn't exist"))),
        }
    }
}

pub struct Transport {
    qp: Arc<QueuePair>,
    endpoint: DatagramEndpoint,
    send_mrs: MrPool,
    recv_mrs: MrPool,
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
        let send_mrs = MrPool::new(Arc::clone(&context), POOL_SIZE)?;
        let mut recv_mrs = MrPool::new(context, POOL_SIZE)?;

        // init post recv
        for _ in 0..POOL_SIZE {
            let (id, mr) = recv_mrs
                .get_free_mr()
                .ok_or_else(|| Error::Internal("no available mr".to_string()))?;
            qp.post_recv(&mr, 0..BUF_SIZE, id)
                .map_err(|err| Error::Internal(alloc::format!("internal error: {err}")))?;
        }

        Ok(Self {
            qp,
            endpoint,
            send_mrs,
            recv_mrs,
        })
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
        let send_mrs = MrPool::new(Arc::clone(&context), POOL_SIZE)?;
        let mut recv_mrs = MrPool::new(context, POOL_SIZE)?;

        // init post recv
        for _ in 0..POOL_SIZE {
            let (id, mr) = recv_mrs
                .get_free_mr()
                .ok_or_else(|| Error::Internal("no available mr".to_string()))?;
            qp.post_recv(&mr, 0..BUF_SIZE, id)
                .map_err(|err| Error::Internal(alloc::format!("internal error: {err}")))?;
        }

        Ok(Self {
            qp,
            endpoint,
            send_mrs,
            recv_mrs,
        })
    }

    pub(crate) fn send<T: Serialize>(&mut self, packets: &[Packet<T>]) -> Result<usize, Error> {
        // poll send cq and update the mr status
        let mut wcs = [Default::default(); POOL_SIZE as usize];
        let res = self
            .qp
            .poll_send_cq(&mut wcs)
            .map_err(|err| Error::Internal(format!("failed to poll send cq, {err}")))?;
        for wc in res {
            self.send_mrs.mark_mr_free(wc.wr_id)?;
        }

        for (i, packet) in packets.iter().enumerate() {
            if let Some((id, mr)) = self.send_mrs.get_free_mr() {
                // serialize arg
                let buffer: &mut [u8] = unsafe {
                    alloc::slice::from_raw_parts_mut(mr.get_virt_addr() as _, BUF_SIZE as usize)
                };
                bincode::serialize_into(buffer, packet)?;
                let size = bincode::serialized_size(packet)?;

                // post send
                self.qp
                    .post_datagram(&self.endpoint, &mr, 0..size, id, true)
                    .map_err(|err| {
                        error!("failed to post datagram: {err}");
                        Error::Internal(err.to_string())
                    })?;
                info!(
                    "transport send packet to remote qpn {:?}",
                    self.endpoint.qpn()
                );
            } else {
                return Ok(packets.len() - i); // such number of packets haven't been sent
            }
        }
        Ok(0) // 0 means all packets have been sent (added to the SQ)
    }

    pub(crate) fn recv<R: DeserializeOwned>(&self) -> Result<Vec<Packet<R>>, Error> {
        // poll recv cq
        let mut wcs = [Default::default(); POOL_SIZE as usize];
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

        let mut packets = Vec::new();
        for wc in res {
            // deserialize arg
            let mr = self.recv_mrs.get_mr_with_id(wc.wr_id)?;
            let msg_sz = wc.byte_len as usize - UD_DATA_OFFSET;
            let msg: Packet<R> = bincode::deserialize(unsafe {
                alloc::slice::from_raw_parts(
                    (mr.get_virt_addr() as usize + UD_DATA_OFFSET) as *mut u8,
                    msg_sz,
                )
            })?; // TODO: post recv when handle error
            packets.push(msg);

            // post recv
            self.qp
                .post_recv(&mr, 0..BUF_SIZE, wc.wr_id)
                .map_err(|err| Error::Internal(alloc::format!("internal error: {err}")))?;
        }

        Ok(packets)
    }

    pub(crate) fn send_all<T: Serialize>(&mut self, packets: Vec<Packet<T>>) -> Result<(), Error> {
        let len = packets.len();
        let mut left_to_be_sent: usize = len;

        loop {
            left_to_be_sent = self.send(&packets[len - left_to_be_sent..])?;
            if left_to_be_sent == 0 {
                break Ok(());
            }
        }
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
