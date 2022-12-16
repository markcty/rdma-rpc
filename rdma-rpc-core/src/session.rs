use core::cmp::Ordering;

use serde::{de::DeserializeOwned, Serialize};
use tracing::info;

use crate::{error::Error, messages::Packet, transport::Transport};

/// Session provides send/receive between server/client
/// Session should act like a stream. Users will read/write from this object.
/// Should handle reorder and package loss.
pub struct Session {
    transport: Transport,
    id: u64,
    syn: u64,
    ack: u64,
}

impl Session {
    // TODO: exchange ack and syn using tcp
    pub fn new(id: u64, transport: Transport) -> Self {
        Self {
            transport,
            id,
            ack: 0,
            syn: 0,
        }
    }

    pub fn id(&self) -> u64 {
        self.syn
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
