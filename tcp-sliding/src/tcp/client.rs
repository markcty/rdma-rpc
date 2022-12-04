use std::io::{Read, Write};
use std::io;
use std::net::TcpStream;
use std::str::from_utf8;
use crate::config::{LOCAL_HOST};

pub fn run_client() {
    match TcpStream::connect(LOCAL_HOST) {
        Ok(mut stream) => {
            println!("Successfully connected to server in {}",LOCAL_HOST);

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
pub fn client_send() {
    let mut stream = TcpStream::connect(LOCAL_HOST).expect("connect failed");

    loop {
        let mut input = String::new();
        let size = io::stdin().read_line(&mut input).expect("read line failed");

        stream
            .write(&input.as_bytes()[..size])
            .expect("write failed");
    }
}
