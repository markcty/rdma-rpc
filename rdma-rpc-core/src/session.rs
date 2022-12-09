use serde::{de::DeserializeOwned, Serialize};

use crate::{error::Error, messages::Packet, transport::Transport};

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
        self.transport.send(self.id, data)?;
        Ok(())
    }

    /// Return true if the packet is the expected one
    pub(crate) fn recv<R: DeserializeOwned>(&self) -> Result<Packet<R>, Error> {
        let packet = self.transport.recv()?;
        assert_eq!(packet.session_id(), self.id);

        // TODO: handle reorder and lost

        Ok(packet)
    }
}
