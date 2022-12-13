use alloc::sync::Arc;
use serde::{de::DeserializeOwned, Serialize};
use KRdmaKit::{context::Context, QueuePair};

use crate::{error::Error, messages::QPInfo, session::Session, transport::Transport};

pub struct ClientStub {
    session: Session,
}

impl ClientStub {
    pub fn new(
        qp: Arc<QueuePair>,
        context: Arc<Context>,
        qp_info: QPInfo,
        session_id: u64,
        port: u8,
    ) -> Result<Self, Error> {
        let transport = Transport::new_with_qp(qp, context, qp_info, port)?;
        let session = Session::new(session_id, transport);
        Ok(Self { session })
    }

    pub fn sync_call<T: Serialize, R: DeserializeOwned>(&self, args: T) -> Result<R, Error> {
        // remote call
        self.session.send(args)?;
        Ok(self.session.recv()?.into_inner())
    }
}
