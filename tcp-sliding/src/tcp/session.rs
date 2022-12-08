use bytes::{BufMut, BytesMut};

use super::types::{deserialize_message, serialize_any, Response, RESPONSE_SIZE};
use crate::config::{SendCase, CONNECT_PASSWORD, END_ACK};
use crate::config::{MAX_BUFF, SEND_CASE};
use crate::tcp::types::{deserialize_response, Message, MESSAGE_CONTENT_SIZE, MESSAGE_SIZE};
use crate::tcp::utils::{client_prefix, server_prefix};
use std::io::{Read, Write};

use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;
pub struct Session {
    pub stream: TcpStream,
    pub send_case: SendCase,
}

impl Session {
    pub fn new(server_addr: &str) -> Result<Session, ()> {
        match TcpStream::connect(server_addr) {
            Ok(stream) => Ok(Session {
                stream: stream,
                send_case: SendCase::Normal,
            }),
            Err(_) => Err(()),
        }
    }
    pub fn server_build_session(stream: TcpStream) -> Session {
        Session {
            stream: stream,
            send_case: SendCase::Normal,
        }
    }
    pub fn receive(&self, data: &mut BytesMut) -> Result<usize, ()> {
        receive_data(&self.stream, data)
    }
    pub fn receivev(mut self, data: &mut BytesMut) -> Result<usize, ()> {
        let mut cur_data = [0u8; MAX_BUFF];
        let mut data_count = 0;
        while match self.stream.read(&mut cur_data) {
            Ok(_size) => {
                // echo everything!
                let get_message = read_message_from_data(cur_data).unwrap();
                println!("{} => {:?}", server_prefix("get"), get_message);
                if get_message.ack_num == END_ACK {
                    println!("get end ack");
                    return Ok(data_count);
                }
                let resp = Response::new(0, get_message.seq_num);
                data.put(&get_message.content[..]);
                std::thread::sleep(Duration::from_secs(1));
                data_count += MESSAGE_CONTENT_SIZE;
                response_send(&self.stream, resp).unwrap();
                true
            }
            Err(_) => {
                println!(
                    "An error occurred, terminating connection with {}",
                    self.stream.peer_addr().unwrap()
                );
                self.stream.shutdown(Shutdown::Both).unwrap();
                false
            }
        } {}
        Ok(data_count)
    }
    pub fn sendM(&self, data: &BytesMut) -> Result<usize, ()> {
        self.send(&data[..])
    }
    pub fn send(&self, data: &[u8]) -> Result<usize, ()> {
        let msg_total_num = (data.len() + MESSAGE_CONTENT_SIZE - 1) / MESSAGE_CONTENT_SIZE;
        println!("message size = {}", msg_total_num);
        let mut send_count: usize = 0;
        let mut idx: u32 = 0;
        while idx < msg_total_num as u32 {
            let down_bound = idx as usize * MESSAGE_CONTENT_SIZE;
            let up_bound = down_bound as usize + MESSAGE_CONTENT_SIZE;
            println!("down = {}, up = {}", down_bound, up_bound);
            let content = data[down_bound..up_bound]
                .try_into()
                .expect("slice with incorrect length");
            let send_msg = Message::new(idx, 0, content);
            println!("{} msg = {:?}", client_prefix("assemble"), send_msg);
            let send_num = message_send(&self.stream, send_msg)?;
            send_count += send_num;
            let get_resp = read_response(&self.stream)?;
            println!("{} {:?}", client_prefix("get"), get_resp);
            if get_resp.ack_num != send_msg.seq_num {
                continue;
            }
            idx += 1;
        }
        let end_msg = Message::new(0, END_ACK, [0u8; MESSAGE_CONTENT_SIZE]);
        message_send(&self.stream, end_msg).unwrap();
        std::thread::sleep(Duration::from_secs(1));
        Ok(send_count)
    }
}

pub fn receive_data(mut stream: &TcpStream, data: &mut BytesMut) -> Result<usize, ()> {
    let mut cur_data = [0u8; MAX_BUFF];
    let mut data_count = 0;
    while match stream.read(&mut cur_data) {
        Ok(_size) => {
            // echo everything!
            let get_message = read_message_from_data(cur_data).unwrap();
            println!("{} => {:?}", server_prefix("get"), get_message);
            if get_message.ack_num == END_ACK {
                println!("get end ack");
                return Ok(data_count);
            }
            let resp = Response::new(0, get_message.seq_num);
            data.put(&get_message.content[..]);
            std::thread::sleep(Duration::from_secs(1));
            data_count += MESSAGE_CONTENT_SIZE;
            response_send(stream, resp).unwrap();
            true
        }
        Err(_) => {
            println!(
                "An error occurred, terminating connection with {}",
                stream.peer_addr().unwrap()
            );
            stream.shutdown(Shutdown::Both).unwrap();
            return Err(());
        }
    } {}
    Ok(data_count)
}
pub fn message_send(mut stream: &TcpStream, msg: Message) -> Result<usize, ()> {
    let msg_serial = unsafe { serialize_any(&msg) };
    match SEND_CASE {
        SendCase::Normal => match stream.write(msg_serial) {
            Ok(num) => Ok(num),
            Err(_) => Err(()),
        },
        SendCase::MayLoss => Ok(0),
        SendCase::MayOverTime => Ok(0),
    }
}
pub fn response_send(mut stream: &TcpStream, response: Response) -> Result<usize, ()> {
    let msg_serial = unsafe { serialize_any(&response) };
    match SEND_CASE {
        SendCase::Normal => match stream.write(msg_serial) {
            Ok(num) => Ok(num),
            Err(_) => Err(()),
        },
        SendCase::MayLoss => Ok(0),
        SendCase::MayOverTime => Ok(0),
    }
}
pub fn read_message_from_data(data: [u8; MAX_BUFF]) -> Result<Message, ()> {
    let get_msg = unsafe { deserialize_message(&data) };
    if !get_msg.check_checksum() {
        println!("{}  = {:?}", server_prefix("error get msg"), get_msg);
        return Err(());
    }
    Ok(get_msg)
}
pub fn read_message(mut stream: &TcpStream) -> Result<Message, ()> {
    let mut data = [0 as u8; MAX_BUFF];
    match stream.read(&mut data) {
        Ok(_) => {
            let get_msg = unsafe { deserialize_message(&data) };
            if !get_msg.check_checksum() {
                println!("{}  = {:?}", server_prefix("error get msg"), get_msg);
                return Err(());
            }
            Ok(get_msg)
        }
        Err(_) => Err(()),
    }
}

pub fn read_response(mut stream: &TcpStream) -> Result<Response, ()> {
    let mut data = [0 as u8; MAX_BUFF];
    match stream.read(&mut data) {
        Ok(_) => {
            let get_resp = unsafe { deserialize_response(&data) };
            if !get_resp.check_checksum() {
                println!("{} = {:?}", client_prefix("error get resp"), get_resp);
                return Err(());
            }
            println!("{}", client_prefix("checksum OK"));
            Ok(get_resp)
        }
        Err(_) => Err(()),
    }
}
