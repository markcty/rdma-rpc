use crate::config::{LOCAL_HOST, MAX_MSG_SIZE};
use crate::tcp::utils::client_prefix;
use bytes::BytesMut;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::str::from_utf8;

use super::session::Session;

pub fn client_send_test() {
    let session = Session::new(LOCAL_HOST).expect("session connect failed");
    let test_data = &[1u8; 24];
    println!("test data = {:?}", test_data);
    let send_res = session.send(test_data).unwrap();
    // std::thread::sleep(Duration::from_secs(10));
    println!("send [size = {}] data success", send_res);
    let mut response_data = BytesMut::with_capacity(MAX_MSG_SIZE);
    println!("{}", client_prefix("start waiting"));
    session.receive(&mut response_data).unwrap();

    return;
}
