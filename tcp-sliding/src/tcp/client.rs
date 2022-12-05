use crate::config::LOCAL_HOST;
use crate::tcp::types::{deserialize_row, serialize_row, Response};
use std::io;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::str::from_utf8;

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
    let mut stream = TcpStream::connect(LOCAL_HOST).expect("connect failed");

    for _ in 0..10 {
        // let row = Response { ack: 1 };
        // let msg = serialize_row(&row);
        let msg: &[u8; 8] = &[0, 0, 0, 4, 5, 6, 7, 8];

        stream.write(msg).unwrap();
        println!("Sent Hello, awaiting reply...");

        let mut data = [0 as u8; 4]; // using 6 byte buffer
        match stream.read_exact(&mut data) {
            Ok(_) => {
                println!("{:?}", String::from_utf8((&data).to_vec()).unwrap());

                // let bar = data.clone();
                let back_to_u32: u32 = u32::from_be_bytes(data);
                println!("{}", back_to_u32);
                // let get: u32 = data.try_into().unwrap();
                // if &data == msg {
                // println!("Reply is ok!");
                // } else {
                // let text = from_utf8(&data).unwrap();
                // println!("Unexpected reply: {}", text);
                // }
            }
            Err(e) => {
                println!("Failed to receive data: {}", e);
            }
        }
        match stream.read_exact(&mut data) {
            Ok(_) => {
                println!("{:?}", String::from_utf8((&data).to_vec()).unwrap());
            }
            Err(e) => {
                println!("Failed to receive data: {}", e);
            }
        }
    }
}
