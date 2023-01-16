use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use rdma_rpc_core::{
    client_stub::ClientStub,
    messages::QPInfo,
    server_stub::{RpcHandler, ServerStub},
    session::Session,
    transport::Transport,
};
use serde::{Deserialize, Serialize};
use KRdmaKit::{services_user, QueuePairBuilder, UDriver};

fn main() {
    let (s1, s2): (Session, Session) = create_session_pair();

    let mut client = ClientStub::new(s1);
    let server = ServerStub::new(s2, Arc::new(KVRpcHandler::new()));

    std::thread::spawn(move || server.serve());

    let resp: Resp = client.sync_call(Args::Put(1, 1)).unwrap();
    assert!(matches!(resp, Resp::Put));

    loop {
        let resp: Resp = client.sync_call(Args::Get(1)).unwrap();
        let i = if let Resp::Get(Some(i)) = resp {
            i
        } else {
            panic!("something went wrong");
        };
        let resp: Resp = client.sync_call(Args::Put(1, i + 1)).unwrap();
        assert!(matches!(resp, Resp::Put));
        println!("{i}");

        std::thread::sleep(Duration::from_millis(500));
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Args {
    Get(i32),
    Put(i32, i32),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Resp {
    Get(Option<i32>),
    Put,
}

struct KVRpcHandler {
    store: Arc<Mutex<HashMap<i32, i32>>>,
}

impl RpcHandler for KVRpcHandler {
    type Args = Args;

    type Resp = Resp;

    fn handle(&self, arg: Self::Args) -> Self::Resp {
        let mut store = self.store.lock().unwrap();
        match arg {
            Args::Get(k) => Resp::Get(store.get(&k).cloned()),
            Args::Put(k, v) => {
                store.insert(k, v);
                Self::Resp::Put
            }
        }
    }
}

impl KVRpcHandler {
    fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

fn create_session_pair() -> (Session, Session) {
    let udriver = UDriver::create().unwrap();
    let device = udriver.devices().get(0).unwrap();
    let context = device.open_context().unwrap();

    let qp1 = QueuePairBuilder::new(&context)
        .build_ud()
        .unwrap()
        .bring_up_ud()
        .unwrap();
    let qp_info1 = QPInfo {
        lid: qp1.lid().unwrap(),
        gid: services_user::ibv_gid_wrapper::from(qp1.gid().unwrap()),
        qp_num: qp1.qp_num(),
        qkey: qp1.qkey(),
    };
    let qp2 = QueuePairBuilder::new(&context)
        .build_ud()
        .unwrap()
        .bring_up_ud()
        .unwrap();
    let qp_info2 = QPInfo {
        lid: qp2.lid().unwrap(),
        gid: services_user::ibv_gid_wrapper::from(qp2.gid().unwrap()),
        qp_num: qp2.qp_num(),
        qkey: qp2.qkey(),
    };

    let tp1 = Transport::new_with_qp(qp1, Arc::clone(&context), qp_info2, 1).unwrap();
    let tp2 = Transport::new_with_qp(qp2, Arc::clone(&context), qp_info1, 1).unwrap();
    (Session::new(0, tp1), Session::new(0, tp2))
}
