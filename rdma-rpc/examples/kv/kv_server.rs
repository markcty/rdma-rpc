use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use protocol::{Args, Resp};
use rdma_rpc::Server;
use rdma_rpc_core::server_stub::RpcHandler;

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
    Server::new(
        "rxe0",
        1,
        "0.0.0.0:0".parse().unwrap(),
        Arc::new(KVRpcHandler::new()),
    )
    .unwrap()
    .serve();
}
