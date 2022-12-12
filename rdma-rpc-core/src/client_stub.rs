use alloc::sync::Arc;
use serde::{de::DeserializeOwned, Serialize};
use KRdmaKit::{
    log::{info, warn},
    MemoryRegion, QueuePair, QueuePairBuilder, UDriver,
};

use crate::{
    error::Error,
    messages::{QPInfo, BUF_SIZE},
    session::Session,
    transport::Transport,
    utils::poll_packets,
};

pub struct ClientStub {
    session: Session,
    qp: Arc<QueuePair>,
    mr: Arc<MemoryRegion>,
}

impl ClientStub {
    pub fn new(dev: &str, qp_info: QPInfo, session_id: u64) -> Result<Self, Error> {
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
        // for the MR, its layout is:
        // |0    ... 1024 | // send buffer
        // |1024 ... 2048 | // receive buffer
        let mr = Arc::new(
            MemoryRegion::new(Arc::clone(&ctx), BUF_SIZE as usize).expect("failed to allocate MR"),
        );
        let transport =
            Transport::new(Arc::clone(&qp), qp_info, Arc::clone(&ctx), Arc::clone(&mr))?;
        let session = Session::new(session_id, transport);
        Ok(Self { session, qp, mr })
    }

    pub fn sync_call<T: Serialize, R: DeserializeOwned>(&self, args: T) -> Result<R, Error> {
        // remote call
        self.session.send(args)?;
        loop {
            let packets = poll_packets(self.qp.as_ref(), Arc::clone(&self.mr))?;
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
