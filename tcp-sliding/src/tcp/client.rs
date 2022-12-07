use crate::config::LOCAL_HOST;
use crate::tcp::types::*;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::str::from_utf8;

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
    let session = Session::new(LOCAL_HOST);
    let test_data = &[1u8; 12];
    println!("test data = {:?}", test_data);
    session.send(test_data);
    return;
}
