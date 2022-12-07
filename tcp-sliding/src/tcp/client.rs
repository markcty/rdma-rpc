use crate::config::LOCAL_HOST;
use crate::tcp::types::*;
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
pub enum SendCase {
    Normal,
    MayLoss,
    MayOverTime,
}
pub fn down_stream_send(mut stream: &TcpStream, context: &[u8], mode: &SendCase) {
    match mode {
        SendCase::Normal => {
            stream.write(context).unwrap();
        }
        SendCase::MayLoss => {}
        SendCase::MayOverTime => {}
    };
}
pub struct Frame {}
pub struct Row {
    id: u32,
    username: [u8; 32],
    email: [u8; 255],
}
pub fn client_send_test() {
    let mut stream = TcpStream::connect(LOCAL_HOST).expect("connect failed");
    let send_mode = SendCase::Normal;
    for i in 0..1 {
        // let row = Response { ack: 1 };
        let msg_str = Message {
            id: i as u32,
            context: [1; 16],
        };
        let msg = unsafe { serialize_any(&msg_str) };
        println!("msg = {:?}, len = {:?}", msg, msg.len());
        down_stream_send(&stream, msg, &send_mode);
        // stream.write(msg).unwrap();

        let mut data = [0 as u8; RESPONSE_SIZE]; // using 6 byte buffer
        match stream.read(&mut data) {
            Ok(_) => {
                let get_resp = unsafe { deserialize_response(&data) };
                println!("{:?}", get_resp);
            }
            Err(e) => {
                println!("Failed to receive data: {}", e);
            }
        }
        // match stream.read_exact(&mut data) {
        //     Ok(_) => {
        //         println!("{:?}", String::from_utf8((&data).to_vec()).unwrap());
        //     }
        //     Err(e) => {
        //         println!("Failed to receive data: {}", e);
        //     }
        // }
    }
}
