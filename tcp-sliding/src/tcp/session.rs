use crate::config::SendCase;
use crate::config::{FRAME_CONTENT_MAX_LEN, SEND_CASE};
use crate::tcp::types::{deserialize_response, Message, MESSAGE_CONTENT_SIZE, MESSAGE_SIZE};
use std::io::{Read, Write};
use std::net::TcpStream;

use super::types::{serialize_any, Response, RESPONSE_SIZE};
pub struct Session {
    pub stream: TcpStream,
    pub send_case: SendCase,
}

impl Session {
    pub fn new(server_addr: &str) -> Session {
        Session {
            stream: TcpStream::connect(server_addr).expect("connect failed"),
            send_case: SendCase::Normal,
        }
    }
    pub fn send(self, data: &[u8]) {
        let msg_total_num = (data.len() + MESSAGE_CONTENT_SIZE - 1) / MESSAGE_CONTENT_SIZE;
        println!("message size = {}", msg_total_num);
        let mut idx: u32 = 0;
        while idx < msg_total_num as u32 {
            let down_bound = idx as usize * MESSAGE_CONTENT_SIZE;
            let up_bound = down_bound as usize + MESSAGE_CONTENT_SIZE;
            println!("down = {}, up = {}", down_bound, up_bound);
            let send_msg = Message {
                id: idx,
                content: data[down_bound..up_bound]
                    .try_into()
                    .expect("slice with incorrect length"),
            };
            println!("assemble msg = {:?}", send_msg);
            message_send(&self.stream, send_msg);
            let get_resp = wait_response(&self.stream).unwrap();
            println!("[client][get] {:?}", get_resp);
            if get_resp.ack != send_msg.id {
                continue;
            }
            idx += 1;
        }
    }
}
fn message_send(mut stream: &TcpStream, msg: Message) {
    let msg_serial = unsafe { serialize_any(&msg) };
    match SEND_CASE {
        SendCase::Normal => {
            stream.write(msg_serial).unwrap();
        }
        SendCase::MayLoss => {}
        SendCase::MayOverTime => {}
    }
}
fn wait_response(mut stream: &TcpStream) -> Result<Response, ()> {
    let mut data = [0 as u8; RESPONSE_SIZE];
    match stream.read(&mut data) {
        Ok(_) => {
            let get_resp = unsafe { deserialize_response(&data) };
            Ok(get_resp)
        }
        Err(_) => Err(()),
    }
}
