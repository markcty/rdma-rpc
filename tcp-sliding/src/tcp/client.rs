use crate::config::{LOCAL_HOST, MAX_MSG_SIZE};
use crate::tcp::types::*;
use crate::tcp::utils::client_prefix;
use bytes::BytesMut;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::str::from_utf8;
use std::time::Duration;

use super::session::Session;

pub fn run_client() {
    match TcpStream::connect(LOCAL_HOST) {
        Ok(mut stream) => {
            println!("Successfully connected to server in {}", LOCAL_HOST);

            let msg = b"Hello!";

            stream.write(msg).unwrap();
            println!("Sent Hello, awaiting reply...");

            let mut data = [0 as u8; 6]; // using 6 byte buffer
            match stream.read_exact(&mut data) {
                Ok(_) => {
                    if &data == msg {
                        println!("Reply is ok!");
                    } else {
                        let text = from_utf8(&data).unwrap();
                        println!("Unexpected reply: {}", text);
                    }
                }
                Err(e) => {
                    println!("Failed to receive data: {}", e);
                }
            }
        }
        Err(e) => {
            println!("Failed to connect: {}", e);
        }
    }
    println!("Terminated.");
}
pub fn client_send_test() {
    let session = Session::new(LOCAL_HOST).expect("session connect failed");
    let test_data = &[1u8; 8];
    println!("test data = {:?}", test_data);
    let send_res = session.send(test_data).unwrap();
    // std::thread::sleep(Duration::from_secs(10));
    println!("send [size = {}] data success", send_res);
    let mut response_data = BytesMut::with_capacity(MAX_MSG_SIZE);
    println!("{}", client_prefix("start waiting"));
    session.receive(&mut response_data).unwrap();

    return;
}
