extern crate alloc;

use crate::{error::Error, session::Session};
use alloc::{collections::BTreeMap, sync::Arc};
use serde::{de::DeserializeOwned, Serialize};
use spin::Mutex;
use KRdmaKit::{log::warn, QueuePair};

pub trait RpcHandler: Send + Sync {
    type Args;
    type Resp;
    fn handle(&self, arg: Self::Args) -> Self::Resp;
}

pub struct ServerStub<T, R> {
    session: Session,
    qp: Arc<QueuePair>,
    handler: Arc<dyn RpcHandler<Args = T, Resp = R>>,
}

impl<T, R> ServerStub<T, R>
where
    T: DeserializeOwned,
    R: Serialize,
{
    /// Session is maintained by the outside because in no_std, we cannot spawn another thread to handle incomming connection
    pub fn new(session: Session, handler: Arc<dyn RpcHandler<Args = T, Resp = R>>) -> Self {
        // create context and qp
        todo!()
    }

    pub fn serve(self) {
        loop {
            // validate the packet
            let packet = match self.session.recv() {
                Err(err) => {
                    warn!("failed to recv new request, {err}");
                    continue;
                }
                Ok(packet) => packet,
            };

            let args = packet.into_inner();
            let resp = self.handler.handle(args);

            // send back the response
            if let Err(e) = self.session.send(resp) {
                warn!("failed to send response, {e}");
            }
        }
    }
}
