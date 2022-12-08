use bytes::BytesMut;

use crate::config::{END_ACK, LOCAL_HOST, MAX_BUFF, MAX_MSG_SIZE};
use crate::tcp::session::{message_send, read_message_from_data, response_send};
use crate::tcp::types::*;
use crate::tcp::utils::server_prefix;
use std::io::{Read, Write};
use std::isize::MAX;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use super::session::{self, read_message, Session};
pub fn run_server_background() {
    thread::spawn(|| {
        tcp_listener();
    });
}
fn handle_client(mut stream: TcpStream) {
    let mut session = Session::server_build_session(stream);
    let mut request_data = BytesMut::with_capacity(MAX_MSG_SIZE);
    session.receive(&mut request_data).unwrap();
    println!("received data = {:?}", &request_data[..]);
    std::thread::sleep(Duration::from_secs(2));
    session.sendM(&request_data);
    
}
fn handle_clientv(mut stream: TcpStream, mut request_data: &[u8]) {
    request_data = &[9u8; MAX_BUFF];
    let mut data = [0 as u8; MAX_BUFF]; // using 50 byte buffer
    while match stream.read(&mut data) {
        Ok(_size) => {
            // echo everything!
            let get_message = read_message_from_data(data).unwrap();
            println!("{} => {:?}", server_prefix("get"), get_message);
            if get_message.ack_num == END_ACK {
                println!("get end ack");
                return;
            }
            let resp = Response::new(0, get_message.seq_num);
            std::thread::sleep(Duration::from_secs(1));
            response_send(&stream, resp).unwrap();
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
                    // let mut request_data = &[0u8; MAX_BUFF];
                    // handle_client(stream, request_data);
                    // println!("{} {:?}", server_prefix("request_get"), request_data);
                    handle_client(stream);
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
