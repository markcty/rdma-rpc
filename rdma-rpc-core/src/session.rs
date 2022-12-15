use serde::{de::DeserializeOwned, Serialize};

use crate::{error::Error, messages::Packet, transport::Transport};

const WINDOW_SIZE: usize = 4;
const SIZE_ACK: usize = 4;
const END_ACK: usize = 99;
/// Session provides send/receive between server/client
/// Session should act like a stream. Users will read/write from this object.
/// Should handle reorder and package loss.
pub struct Session {
    transport: Transport,
    id: u64,
}

impl Session {
    pub fn new(id: u64, transport: Transport) -> Self {
        Self { transport, id }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub(crate) fn send<T: Serialize>(&self, data: T) -> Result<(), Error> {
        // TODO: devide data into multiple packets if needed
        // let packet = Packet::new(self.id, data);
        // self.transport.send(packet)?;
        // Ok(())
        let packet = Packet::new(0, 0, self.id, data);
        self.transport.send(packet)?;
        Ok(())
    }

    /// Return true if the packet is the expected one
    pub(crate) fn recv<R: DeserializeOwned>(&self) -> Result<R, Error> {
        // TODO: assemble the packets to R

        // let mut window_base: u32 = 0;
        // let accept_range: [bool; WINDOW_SIZE] = [true; WINDOW_SIZE];
        // loop {
        //     let packet = self.transport.recv()?;
        //     if !packet.check_checksum() || packet.session_id() != self.id {
        //         continue;
        //     }
        //     println!(
        //         "[server][get] => {:?}, window_base = {}, accept_range = {:?}",
        //         packet, window_base, accept_range
        //     );
        //     match packet.ack_num() {
        //         END_ACK => {}
        //         _ => {}
        //     }
        // }
        let packet = self.transport.recv()?;
        assert_eq!(packet.session_id(), self.id);
        // // TODO: handle reorder and lost

        Ok(packet.into_inner())
    }
}
