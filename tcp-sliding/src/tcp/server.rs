use crate::config::LOCAL_HOST;
use crate::tcp::types::*;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;
pub fn run_server_background() {
    thread::spawn(|| {
        tcp_listener();
    });
}

fn handle_client(mut stream: TcpStream) {
    let mut data = [0 as u8; MESSAGE_SIZE]; // using 50 byte buffer
    while match stream.read(&mut data) {
        Ok(size) => {
            // echo everything!
            let get_message = unsafe { deserialize_message(&data) };
            println!("[server][get] => {:?}", get_message);
            let resp = Response {
                ack: get_message.id,
            };
            let reply_data = unsafe { serialize_any(&resp) };
            std::thread::sleep(Duration::from_secs(1));
            stream.write(&reply_data).unwrap();
            true
        }
        Err(_) => {
            println!(
                "An error occurred, terminating connection with {}",
                stream.peer_addr().unwrap()
            );
            stream.shutdown(Shutdown::Both).unwrap();
            false
        }
    } {}
}

fn tcp_listener() {
    let listener = TcpListener::bind(LOCAL_HOST).unwrap();
    // accept connections and process them, spawning a new thread for each one
    println!("Server listening on {}\n", LOCAL_HOST);
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New connection: {}", stream.peer_addr().unwrap());
                thread::spawn(move || {
                    // connection succeeded
                    handle_client(stream)
                });
            }
            Err(e) => {
                println!("Error: {}", e);
                /* connection failed */
            }
        }
    }
    // close the socket server
    drop(listener);
}
