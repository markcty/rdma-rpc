use alloc::{format, string::ToString, sync::Arc};
use serde::{de::DeserializeOwned, Serialize};

use crate::{error::Error, messages::Packet, transport::Transport};
use tracing::{info, warn};
use KRdmaKit::{
    context::Context,
    services_user::{self},
    DatagramEndpoint, MemoryRegion, QueuePair, QueuePairBuilder,
};

// for the MR, its layout is:
// |0    ... 4096 | // send buffer
// |4096 ... 8192 | // receive buffer
const DATA_SIZE: usize = 4096;
const SESSION_START: usize = 2048;
const PAKET_HEADER: usize = 256;
const MESSAGE_CONTENT_SIZE: usize = 40;
/// Session provides send/receive between server/client
/// Session should act like a stream. Users will read/write from this object.
/// Should handle reorder and package loss.
pub struct Session {
    transport: Transport,
    id: u64,
    pub mr: MemoryRegion,
}

impl Session {
    pub fn new(context: Arc<Context>, id: u64, transport: Transport) -> Self {
        // create mr
        let mr = MemoryRegion::new(Arc::clone(&context), DATA_SIZE as usize)
            .map_err(|err| Error::Internal(format!("failed to allocate MR, {err}")))
            .unwrap();

        Self { transport, id, mr }
    }

    pub fn id(&self) -> u64 {
        self.id
    }
    fn get_session_addr(&self) -> usize {
        self.mr.get_virt_addr() as usize
    }
    pub(crate) fn send_u8<T: Serialize>(&self, data: T) -> Result<u64, Error> {
        let packet = Packet::new(self.id, data);
        let send_buffer: &mut [u8] = unsafe {
            alloc::slice::from_raw_parts_mut(self.get_session_addr() as _, DATA_SIZE as usize)
        };
        bincode::serialize_into(send_buffer, &packet)?;
        let size = bincode::serialized_size(&packet)?;
        self.transport.send_u8(&self.mr, 0, size)?;

        // loop {
        //     let packet: Result<Packet<R>, Error> = self.transport.recv_fa();
        //     match packet {
        //         Ok(_pck) => {
        //             break;
        //         }
        //         Err(Error::Receive) => {
        //             continue;
        //         }
        //         Err(err) => {
        //             return Err(err);
        //         }
        //     }
        // }
        Ok(size)
    }

    pub(crate) fn send<T: Serialize>(&self, data: T) -> Result<(), Error> {
        // TODO: devide data into multiple packets if needed
        let packet = Packet::new(self.id, data);
        self.transport.send(packet)?;
        Ok(())
    }

    /// Return true if the packet is the expected one
    pub(crate) fn recv<R: DeserializeOwned>(&self) -> Result<R, Error> {
        // TODO: assemble the packets to R
        let packet = self.transport.recv()?;
        assert_eq!(packet.session_id(), self.id);
        let resp_packet = Packet::new(packet.session_id(), [0u8; 0]);
        // self.transport.send(resp_packet)?;
        // TODO: handle reorder and lost
        Ok(packet.into_inner())
    }
}
