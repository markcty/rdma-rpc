extern crate alloc;

use alloc::sync::Arc;

use serde::{de::DeserializeOwned, Serialize};
use tracing::{info, warn};

use crate::session::Session;

pub trait RpcHandler: Send + Sync {
    type Args;
    type Resp;
    fn handle(&self, arg: Self::Args) -> Self::Resp;
}

pub struct ServerStub<T, R> {
    session: Session,
    handler: Arc<dyn RpcHandler<Args = T, Resp = R>>,
}

impl<T, R> ServerStub<T, R>
where
    T: DeserializeOwned + Clone,
    R: Serialize + Clone,
{
    pub fn new(session: Session, handler: Arc<dyn RpcHandler<Args = T, Resp = R>>) -> Self {
        Self { session, handler }
    }

    pub fn serve(mut self) -> ! {
        loop {
            // validate the packet
            let args = match self.session.recv() {
                Err(err) => {
                    warn!("failed to recv new request, {err}");
                    continue;
                }
                Ok(packet) => packet,
            };
            info!("new request from client");

            // handle the request
            let resp = self.handler.handle(args);

            // send back the response
            if let Err(e) = self.session.send(resp) {
                warn!("failed to send response, {e}");
            }
        }
    }
}
