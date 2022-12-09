use rdma_rpc::Client;

mod protocol;
use protocol::{Args, Resp};

fn main() {
    let client: Client<Args, Resp> = Client::new();
    println!("{:?}", client.send(Args::Put(1, 1)).unwrap());
    println!("{:?}", client.send(Args::Get(1)).unwrap());
}
