extern crate alloc;

use crate::{error::Error, messages::BUF_SIZE, session::Session, utils::poll_packets};
use alloc::{collections::BTreeMap, sync::Arc};
use serde::{de::DeserializeOwned, Serialize};
use spin::Mutex;
use KRdmaKit::{
    context::Context,
    log::{info, warn},
    MemoryRegion, QueuePair, QueuePairBuilder, UDriver,
};

pub trait RpcHandler: Send + Sync {
    type Args;
    type Resp;
    fn handle(&self, arg: Self::Args) -> Self::Resp;
}

pub struct ServerStub<T, R> {
    sessions: Arc<Mutex<BTreeMap<u64, Session>>>,
    qp: Arc<QueuePair>,
    handler: Arc<dyn RpcHandler<Args = T, Resp = R>>,
    ctx: Arc<Context>,
    mr: Arc<MemoryRegion>,
}

impl<T, R> ServerStub<T, R>
where
    T: DeserializeOwned,
    R: Serialize,
{
    /// Session is maintained by the outside because in no_std, we cannot spawn another thread to handle incomming connection
    pub fn new(
        dev: &str,
        ib_port: u8,
        sessions: Arc<Mutex<BTreeMap<u64, Session>>>,
        handler: Arc<dyn RpcHandler<Args = T, Resp = R>>,
    ) -> Result<Self, Error> {
        // create context
        let ctx = {
            let udriver = UDriver::create().expect("no device");
            let device = if let Some(device) = udriver.devices().iter().find(|d| d.name() == dev) {
                device
            } else {
                return Err(Error::Internal(alloc::format!("can't find device {dev}")));
            };
            device.open_context().expect("can't open context")
        };

        // create qp
        let qp = {
            let builder = QueuePairBuilder::new(&ctx);
            let qp = builder
                .build_ud()
                .expect("failed to build UD QP")
                .bring_up_ud()
                .expect("failed to bring up UD QP");
            info!("QP status: {:?}", qp.status());
            qp
        };
        info!("QP num: {:?}, qkey: {:?}", qp.qp_num(), qp.qkey());

        // create mr
        // for the MR, its layout is:
        // |0    ... 1024 | // send buffer
        // |1024 ... 2048 | // receive buffer
        let mr = Arc::new(
            MemoryRegion::new(Arc::clone(&ctx), BUF_SIZE as usize).expect("failed to allocate MR"),
        );

        Ok(Self {
            sessions,
            qp,
            handler,
            ctx,
            mr,
        })
    }

    pub fn add_session(&self, session: Session) {
        let id = session.id();
        assert!(
            self.sessions.lock().insert(session.id(), session).is_none(),
            "duplicated session {id}",
        );
    }

    pub fn remove_session(&self, session_id: u64) {
        assert!(
            self.sessions.lock().remove(&session_id).is_some(),
            "no session {session_id}",
        );
    }

    pub fn serve(self) {
        loop {
            // poll
            let packets = poll_packets(self.qp.as_ref(), Arc::clone(&self.mr)).unwrap();

            for packet in packets {
                // dispatch the packet to its session
                let session_id = packet.session_id();

                // get the corresponding session
                let sessions = self.sessions.lock();
                let session = if let Some(session) = sessions.get(&session_id) {
                    session
                } else {
                    warn!("no such session {session_id}");
                    continue;
                };

                // validate the packet
                if let Err(err) = session.recv(&packet) {
                    warn!("the packet is unexpected, {err}");
                    continue;
                }

                let args = packet.into_inner();
                let resp = self.handler.handle(args);

                // send back the response
                if let Err(e) = session.send(resp) {
                    warn!("failed to send response to session {}, {e}", session.id());
                }
            }
        }
    }

    pub fn qp(&self) -> Arc<QueuePair> {
        Arc::clone(&self.qp)
    }

    pub fn ctx(&self) -> Arc<Context> {
        Arc::clone(&self.ctx)
    }

    pub fn mr(&self) -> Arc<MemoryRegion> {
        Arc::clone(&self.mr)
    }
}
