use serde::{de::DeserializeOwned, Serialize};
use KRdmaKit::{context::Context, QueuePair};

use crate::{error::Error, messages::QPInfo, session::Session};

pub struct ClientStub {
    session: Session,
}

impl ClientStub {
    pub fn new(context: &Context, qp_info: QPInfo, session_id: u64) -> Result<Self, Error> {
        // a qp, and a session
        todo!();
    }

    pub fn sync_call<T: Serialize, R: DeserializeOwned>(&self, args: T) -> Result<R, Error> {
        // remote call
        self.session.send(args)?;
        Ok(self.session.recv()?.into_inner())
    }
}
