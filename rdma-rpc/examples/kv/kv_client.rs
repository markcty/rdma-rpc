use rdma_rpc::Client;

mod protocol;
use protocol::{Args, Resp};

fn main() {
    let client: Client<Args, Resp> =
        Client::new("rxe_0", "127.0.0.1:10001".parse().unwrap(), 1).unwrap();
    println!("{:?}", client.send(Args::Put(1, 1)).unwrap());
    println!("{:?}", client.send(Args::Get(1)).unwrap());
}
