use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use protocol::{Args, Resp};
use rdma_rpc::Server;
use rdma_rpc_core::server_stub::RpcHandler;
use tracing::Level;

mod protocol;

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

fn main() {
    tracing_subscriber::fmt::fmt()
        .without_time()
        .with_max_level(Level::DEBUG)
        .init();
    Server::new(
        "rxe_0",
        1,
        "127.0.0.1:10001".parse().unwrap(),
        Arc::new(KVRpcHandler::new()),
    )
    .unwrap()
    .serve()
    .unwrap();
}
