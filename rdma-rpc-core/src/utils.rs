use alloc::{sync::Arc, vec::Vec};
use serde::de::DeserializeOwned;
use KRdmaKit::{MemoryRegion, QueuePair};

use crate::{error::Error, messages::Packet};

const UD_DATA_OFFSET: usize = 40; // For a UD message, the first 40 bytes are reserved for GRH

pub(crate) fn poll_packets<T: DeserializeOwned>(
    qp: &QueuePair,
    mr: Arc<MemoryRegion>,
) -> Result<Vec<Packet<T>>, Error> {
    let mut packets = Vec::new();
    // poll one wc every time
    let mut wcs = [Default::default()];
    let res = qp.poll_recv_cq(&mut wcs).expect("failed to poll recv CQ");
    if !res.is_empty() {
        let msg_sz = res[0].byte_len as usize - UD_DATA_OFFSET;
        let msg: Packet<T> = bincode::deserialize(unsafe {
            alloc::slice::from_raw_parts(
                (mr.get_virt_addr() as usize + UD_DATA_OFFSET) as *mut u8,
                msg_sz,
            )
        })
        .map_err(|_| {
            qp.post_recv(mr.as_ref(), 1024..2048, 1)
                .expect("failed to post recv");
            Error::DecodeArgs
        })?;
        packets.push(msg);
        qp.post_recv(mr.as_ref(), 1024..2048, 1)
            .map_err(|err| Error::Internal(alloc::format!("internal error: {err}")))?;
    }

    Ok(packets)
}
