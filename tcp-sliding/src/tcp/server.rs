use bytes::BytesMut;

use crate::config::{LOCAL_HOST, MAX_MSG_SIZE};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use super::session::Session;
pub fn run_server_background() {
    thread::spawn(|| {
        tcp_listener();
    });
}
fn local_fun_call(data: &mut BytesMut) -> Result<&mut BytesMut, ()> {
    std::thread::sleep(Duration::from_secs(2));
    // echo server
    Ok(data)
}
fn reply_error() {}
fn handle_client(stream: TcpStream) {
    let session = Session::server_build_session(stream);
    let mut request_data = BytesMut::with_capacity(MAX_MSG_SIZE);
    session.receive(&mut request_data).unwrap();
    println!("received data = {:?}", &request_data[..]);

    match local_fun_call(&mut request_data) {
        Ok(response_data) => {
            session.sendBytesMute(&response_data).unwrap();
        }
        Err(_) => reply_error(),
    }
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
