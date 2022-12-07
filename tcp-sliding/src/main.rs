mod config;
mod libs;
mod tcp;
use std::thread;
use std::time;
use tcp::client;
use tcp::server;

use crate::tcp::types::serialize_any;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct Row {
    bar: u32,
    content: &'static [u8],
}
fn ser_test() {
    let data = &[3u8; 100];
    let content: &[u8] = data[0..2].try_into().unwrap();
    println!("{:?}", content);
    let row = Row {
        bar: 12u32,
        content,
    };
    println!("{:?}", row);
    let ser_get = unsafe { serialize_any(&row) };
    println!("{:?}", ser_get);
}

fn server_client_test() {
    server::run_server_background();
    thread::sleep(time::Duration::from_secs(1));
    // client::run_client();
    client::client_send_test();
}

fn main() {
    println!("hello world");
    server_client_test();
    // ser_test();
}
