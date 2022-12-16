use std::thread;
use std::time::Duration;

use rdma_rpc::Client;

mod protocol;
use protocol::{Args, Resp};
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .without_time()
        .with_max_level(Level::DEBUG)
        .init();
    let mut client: Client<Args, Resp> =
        Client::new("rxe_0", "127.0.0.1:10001".parse().unwrap(), 1).unwrap();
    info!("call 1 {:?}", client.send(Args::Put(1, 1)).unwrap());
    // thread::sleep(Duration::from_millis(100));
    // info!("call 2 {:?}", client.send(Args::Get(1)).unwrap());
}
