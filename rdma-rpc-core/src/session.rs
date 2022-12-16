use alloc::{format, string::ToString, sync::Arc};
use core::cmp::Ordering;

use serde::{de::DeserializeOwned, Serialize};
use tracing::info;

use crate::{error::Error, messages::Packet, transport::Transport};
use KRdmaKit::{context::Context, DatagramEndpoint, MemoryRegion, QueuePair, QueuePairBuilder};

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
    syn: u64,
    ack: u64,
}

impl Session {
    // TODO: exchange ack and syn using tcp
    pub fn new(context: Arc<Context>, id: u64, transport: Transport) -> Self {
        // create mr
        let mr = MemoryRegion::new(Arc::clone(&context), DATA_SIZE as usize)
            .map_err(|err| Error::Internal(format!("failed to allocate MR, {err}")))
            .unwrap();

        Self {
            transport,
            id,
            mr,
            ack: 0,
            syn: 0,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }
    fn get_session_addr(&self) -> usize {
        self.mr.get_virt_addr() as usize
    }
    pub(crate) fn send_u8<T: Serialize + Clone>(&mut self, data: T) -> Result<(), Error> {
        // TODO: devide data into multiple packets if needed
        let packet = Packet::new(self.ack, self.syn, self.id, bincode::serialize(&data)?);
        self.transport.send(packet.clone())?;

        // wait until the packet is received by the remote
        loop {
            sleep_millis(10);
            if let Some(packet) = self.transport.try_recv()? {
                if packet.ack() == self.syn + 1 {
                    info!("recv ack {}", packet.ack());
                    self.syn += 1;
                    break Ok(());
                }
            }

            // resend
            self.transport.send(packet.clone())?;
        }
    }
    pub(crate) fn send<T: Serialize + Clone>(&mut self, data: T) -> Result<(), Error> {
        // TODO: devide data into multiple packets if needed
        let packet = Packet::new(self.ack, self.syn, self.id, bincode::serialize(&data)?);
        self.transport.send(packet.clone())?;

        // wait until the packet is received by the remote
        loop {
            sleep_millis(10);
            if let Some(packet) = self.transport.try_recv()? {
                if packet.ack() == self.syn + 1 {
                    info!("recv ack {}", packet.ack());
                    self.syn += 1;
                    break Ok(());
                }
            }

            // resend
            self.transport.send(packet.clone())?;
        }
    }

    /// Recv the next request
    pub(crate) fn recv<R: DeserializeOwned>(&mut self) -> Result<R, Error> {
        let packet = loop {
            // TODO: assemble the packets to R
            let packet = self.transport.recv()?;
            assert_eq!(packet.session_id(), self.id);
            match packet.syn().cmp(&self.ack) {
                // probably the remote end did not received my last ack, resend ack
                Ordering::Less => {
                    self.transport
                        .send(Packet::new_empty(self.ack, self.syn, self.id))?;
                    info!("send back ack {}", self.ack);
                }
                // packet is the expected one, return ack
                Ordering::Equal => {
                    self.ack += 1;
                    self.transport
                        .send(Packet::new_empty(self.ack, self.syn, self.id))?;
                    info!("send back ack {}", self.ack);
                    break packet;
                }
                // do nothing wait for the remote to resend
                Ordering::Greater => {}
            }
        };

        Ok(bincode::deserialize(packet.data())?)
    }
}
fn sleep_millis(duration: u32) {
    unsafe {
        libc::usleep(1000 * duration);
    }
}
