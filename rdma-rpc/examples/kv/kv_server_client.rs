use rdma_rpc::Client;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread, time,
    time::Duration,
};

mod protocol;
use protocol::{Args, Resp};
use rdma_rpc::Server;
use rdma_rpc_core::server_stub::RpcHandler;
use tracing::{info, Level};

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

fn server_listener() {
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
fn start_server() {
    thread::spawn(|| {
        server_listener();
    });
}
fn start_client() {
    tracing_subscriber::fmt::fmt()
        .without_time()
        .with_max_level(Level::DEBUG)
        .init();
    let client: Client<Args, Resp> =
        Client::new("rxe_0", "127.0.0.1:10001".parse().unwrap(), 1).unwrap();
    info!("call 1 {:?}", client.send(Args::Put(1, 1)).unwrap());
    info!("call 2 {:?}", client.send(Args::Get(1)).unwrap());
}
fn main() {
    // println!("hello world");
    start_server();
    thread::sleep(time::Duration::from_secs(3));
    start_client();
    thread::sleep(time::Duration::from_secs(10));
}
