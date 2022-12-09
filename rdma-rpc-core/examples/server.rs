use std::{
    collections::HashMap,
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
};

use rdma_rpc::server_stub::RpcHandler;

struct KVRpc {
    store: Arc<Mutex<HashMap<i32, i32>>>,
}

enum Args {
    Get(i32),
    Put(i32, i32),
}

enum Resp {
    Get(Option<i32>),
    Put,
}

impl RpcHandler for KVRpc {
    type Args = Args;

    type Resp = Resp;

    fn handle(&self, arg: Self::Args) -> Self::Resp {
        let mut store = self.store.lock().unwrap();
        match arg {
            Args::Get(k) => Resp::Get(store.get(&k).cloned()),
            Args::Put(k, v) => {
                store.insert(k, v);
                Resp::Put
            }
        }
    }
}

struct KVRpcServer {}

impl KVRpcServer {
    fn serve() {
        let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

        // while let Ok(incoming in listener.incoming() {
        //     thread::spawn(move || {

        //     })
        // }
    }
}

fn main() {}
