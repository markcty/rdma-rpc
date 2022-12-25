use alloc::{collections::BTreeMap, format, string::ToString, sync::Arc, vec::Vec};

use tracing::{debug, error, info};
use KRdmaKit::{
    context::Context,
    services_user::{self},
    DatagramEndpoint, MemoryRegion, QueuePair, QueuePairBuilder,
};

use crate::{
    error::Error,
    messages::{Packet, QPInfo},
};
pub const MTU: u64 = 1024;
const BUF_SIZE: u64 = MTU; // 4KB
const UD_DATA_OFFSET: usize = 40; // for a UD message, the first 40 bytes are reserved for GRH
const MAX_PACKET_BYTES: usize = BUF_SIZE as usize - UD_DATA_OFFSET;
pub(crate) const MAX_DATA_BYTES: usize = MAX_PACKET_BYTES - 33; // reserve for packet meta
const POOL_SIZE: u8 = 64; // how many mrs are there in a mr pool

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

    pub(crate) fn send(&mut self, packets: &[Packet]) -> Result<usize, Error> {
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
                let size = bincode::serialized_size(packet)?;
                assert!((size as usize) <= MAX_PACKET_BYTES);
                bincode::serialize_into(buffer, packet)?;

                // post send
                debug!("send 1 packet, size: {size}");
                self.qp
                    .post_datagram(&self.endpoint, &mr, 0..size, id, true)
                    .map_err(|err| {
                        error!("failed to post datagram: {err}");
                        Error::Internal(err.to_string())
                    })?;
            } else {
                return Ok(packets.len() - i); // such number of packets haven't been sent
            }
        }
        Ok(0) // 0 means all packets have been sent (added to the SQ)
    }

    pub(crate) fn recv(&self) -> Result<Vec<Packet>, Error> {
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

        let mut packets = Vec::new();
        for wc in res {
            // deserialize arg
            let mr = self.recv_mrs.get_mr_with_id(wc.wr_id)?;
            let msg_sz = wc.byte_len as usize - UD_DATA_OFFSET;
            let msg: Packet = bincode::deserialize(unsafe {
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

        debug!("recv {} packets", packets.len());

        Ok(packets)
    }

    pub(crate) fn try_recv(&self) -> Result<Vec<Packet>, Error> {
        // poll recv cq
        let mut wcs = [Default::default(); POOL_SIZE as usize];
        let res = self
            .qp
            .poll_recv_cq(&mut wcs)
            .map_err(|err| Error::Internal(format!("failed to poll cq, {err}")))?;

        let mut packets = Vec::new();
        for wc in res {
            // deserialize arg
            let mr = self.recv_mrs.get_mr_with_id(wc.wr_id)?;
            let msg_sz = wc.byte_len as usize - UD_DATA_OFFSET;
            let msg: Packet = bincode::deserialize(unsafe {
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

        if !packets.is_empty() {
            debug!("recv {} packets", packets.len());
        }

        Ok(packets)
    }

    pub(crate) fn send_burst(&mut self, packets: Vec<Packet>) -> Result<(), Error> {
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

#[cfg(test)]
mod tests {
    use alloc::vec;

    use crate::{
        messages::Packet,
        transport::MAX_DATA_BYTES,
        utils::{
            sleep_millis,
            tests::{new_random_data, new_two_transport},
        },
    };

    #[test]
    fn it_works() {
        tracing_subscriber::fmt::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_max_level(tracing::Level::DEBUG)
            .init();
        let (mut tp1, tp2) = new_two_transport();

        let data = new_random_data(MAX_DATA_BYTES);
        let packet = Packet::new(0, 0, data.clone());
        tp1.send_burst(vec![packet]).unwrap();

        let mut received_packet = tp2.recv().unwrap();

        assert_eq!(data, received_packet.remove(0).into_data());
    }

    #[test]
    fn try_recv() {
        let (mut tp1, tp2) = new_two_transport();

        let packet = Packet::new(0, 0, new_random_data(64));
        tp1.send_burst(vec![packet.clone()]).unwrap();
        sleep_millis(100);

        let mut received_packet = tp2.recv().unwrap();

        assert_eq!(packet.into_data(), received_packet.remove(0).into_data());
    }

    #[test]
    fn try_recv_not_block() {
        let (mut tp1, tp2) = new_two_transport();

        let packet = Packet::new(0, 0, new_random_data(64));

        assert!(tp2.try_recv().unwrap().is_empty());

        tp1.send_burst(vec![packet.clone()]).unwrap();
        sleep_millis(100);

        let mut received_packet = tp2.recv().unwrap();

        assert_eq!(packet.into_data(), received_packet.remove(0).into_data());
    }
}
