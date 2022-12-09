use alloc::sync::Arc;
use serde::{de::DeserializeOwned, Serialize};
use KRdmaKit::{log::warn, QueuePair};

use crate::{
    error::Error, messages::QPInfo, session::Session, transport::Transport, utils::poll_packets,
};

pub struct ClientStub {
    session: Session,
    qp: Arc<QueuePair>,
}

impl ClientStub {
    pub fn new(qp_info: QPInfo, session_id: u64) -> Result<Self, Error> {
        let transport = Transport::new(qp_info)?;
        let session = Session::new(session_id, transport);
        Ok(Self {
            session,
            qp: todo!(),
        })
    }

    pub fn sync_call<T: Serialize, R: DeserializeOwned>(&self, args: T) -> Result<R, Error> {
        // remote call
        self.session.send(args)?;
        loop {
            let packets = poll_packets(self.qp.as_ref())?;
            for packet in packets {
                if let Err(err) = self.session.recv(&packet) {
                    warn!("the received packet is unexpected, {err}");
                } else {
                    return Ok(packet.into_inner());
                }
            }
        }
    }
}
