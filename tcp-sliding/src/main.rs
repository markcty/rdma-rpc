mod libs;
mod tcp;
mod config;
use std::thread;
use std::time;
use tcp::client;
use tcp::server;

fn main() {
    println!("hello world");
    server::run_server_background();
    thread::sleep(time::Duration::from_secs(1));
    client::run_client();
}
