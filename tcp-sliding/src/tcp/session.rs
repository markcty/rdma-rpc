use super::types::{deserialize_message, serialize_any, Response, RESPONSE_SIZE};
use crate::config::{
    SendCase, CONNECT_PASSWORD, END_ACK, LOSS_RATE, SIZE_ACK, TIME_OUT, WINDOW_SIZE,
};
use crate::config::{MAX_BUFF, SEND_CASE};
use crate::tcp::types::{deserialize_response, Message, MESSAGE_CONTENT_SIZE, MESSAGE_SIZE};
use crate::tcp::utils::*;
use crate::tcp::utils::{client_prefix, server_prefix};
use bytes::{BufMut, BytesMut};
use rand::Rng;
use std::cmp::Ordering;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;
use tokio::time::{self, Instant};
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
    pub fn sendBytesMute(&self, data: &BytesMut) -> Result<usize, ()> {
        self.send(&data[..])
    }
    pub fn send(&self, data: &[u8]) -> Result<usize, ()> {
        send_data(&self.stream, data)
    }
}

pub fn send_data(stream: &TcpStream, data: &[u8]) -> Result<usize, ()> {
    let msg_total_num = (data.len() + MESSAGE_CONTENT_SIZE - 1) / MESSAGE_CONTENT_SIZE;
    println!("message size = {}", msg_total_num);
    stream
        .set_read_timeout(Some(Duration::from_micros(10)))
        .unwrap();

    let mut send_count: usize = 0;
    let mut idx: u32 = 0;
    // while sending_idx < msg_total_num{
    //     if stream.re
    // }
    let mut ack_flag = true;
    let mut start_time = Instant::now();
    while idx < msg_total_num as u32 {
        if ack_flag {
            start_time = Instant::now();
            let down_bound = idx as usize * MESSAGE_CONTENT_SIZE;
            let up_bound = down_bound as usize + MESSAGE_CONTENT_SIZE;
            println!("down = {}, up = {}", down_bound, up_bound);
            let content = data[down_bound..up_bound]
                .try_into()
                .expect("slice with incorrect length");
            let send_msg = Message::new(idx, 0, content);
            println!("{} msg = {:?}", client_prefix("assemble"), send_msg);
            let send_num = message_send(stream, send_msg).unwrap();
            send_count += send_num;
            ack_flag = false;
        } else {
            // let get_resp = read_response_short_time(stream).unwrap();
            let duration = start_time.elapsed();
            if duration.cmp(&TIME_OUT) == Ordering::Greater {
                println!("{}", client_prefix("loss package"));
                ack_flag = true;
                continue;
            }
            match read_response_short_time(stream) {
                Ok(get_resp) => {
                    println!("{} {:?}", client_prefix("get"), get_resp);
                    idx += 1;
                    ack_flag = true;
                }
                Err(_) => {}
            };
        }
    }
    let end_msg = Message::new(0, END_ACK, [0u8; MESSAGE_CONTENT_SIZE]);
    message_send(stream, end_msg).unwrap();
    std::thread::sleep(Duration::from_secs(1));
    Ok(send_count)
}
pub fn receive_data(mut stream: &TcpStream, data: &mut BytesMut) -> Result<usize, ()> {
    stream.set_read_timeout(None).unwrap();
    let mut cur_data = [0u8; MAX_BUFF];
    let mut data_count = 0;
    let mut window_base: u32 = 0;
    let mut accept_range: [bool; WINDOW_SIZE] = [true; WINDOW_SIZE];
    let mut cur_window_size: u32 = WINDOW_SIZE as u32;
    let mut cur_buffer = [0u8; WINDOW_SIZE * MESSAGE_CONTENT_SIZE];
    let mut buffer_left = WINDOW_SIZE;
    let mut total_size: u32 = WINDOW_SIZE as u32;
    while match stream.read(&mut cur_data) {
        Ok(_size) => {
            // echo everything!
            let get_message = read_message_from_data(cur_data).unwrap();
            println!(
                "{} => {:?}, window_base = {} ; accept_range = {:?}",
                server_prefix("get"),
                get_message,
                window_base,
                accept_range,
            );
            match get_message.ack_num {
                SIZE_ACK => {
                    total_size = build_u32_from_u8(&get_message.content[..]).unwrap();
                    if total_size < cur_window_size {
                        // already get all in the first window
                        if buffer_left as u32 + total_size == cur_window_size {
                            data.put(&cur_buffer[0..total_size as usize * MESSAGE_CONTENT_SIZE]);
                        }
                        cur_window_size = total_size;
                    }
                    let resp = Response::new(0, SIZE_ACK);
                    response_send(stream, resp).unwrap();
                }
                END_ACK => {
                    data.put(&cur_buffer[0..(WINDOW_SIZE - buffer_left) * MESSAGE_CONTENT_SIZE]);
                    println!("{}", server_prefix("get end ack"));
                    return Ok(data_count);
                }
                _ => {
                    let cur_seq = get_message.seq_num;
                    // new data frame get
                    if window_base <= cur_seq
                        && cur_seq < window_base + WINDOW_SIZE as u32
                        && accept_range[(cur_seq - window_base) as usize]
                    {
                        // need to save data
                        buffer_left -= 1;
                        accept_range[(cur_seq - window_base) as usize] = false;
                        assemble_cur_buffer(
                            &mut cur_buffer,
                            &get_message.content,
                            (cur_seq - window_base) as usize * MESSAGE_CONTENT_SIZE,
                        );
                        println!("{} {:?}", server_prefix("get new data"), cur_buffer);
                        data_count += MESSAGE_CONTENT_SIZE;
                        if buffer_left == 0 {
                            data.put(&cur_buffer[..]);
                            window_base += WINDOW_SIZE as u32;
                            // if window_base + WINDOW_SIZE as u32 > total_size {
                            //     cur_window_size = total_size - window_base;
                            // }
                            accept_range = [true; WINDOW_SIZE];
                            buffer_left = cur_window_size as usize;
                            cur_buffer = [0u8; WINDOW_SIZE * MESSAGE_CONTENT_SIZE];
                        }
                    }
                    // need to reply ack_num
                    let resp = Response::new(0, get_message.seq_num);
                    response_send(stream, resp).unwrap();
                }
            }

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
    Ok(data_count)
}
pub fn message_send(mut stream: &TcpStream, msg: Message) -> Result<usize, ()> {
    let msg_serial = unsafe { serialize_any(&msg) };
    match SEND_CASE {
        SendCase::Normal => real_send(stream, msg_serial),
        SendCase::MayLoss => {
            let mut rng = rand::thread_rng();
            if rng.gen_range(0..100) > LOSS_RATE {
                real_send(stream, msg_serial)
            } else {
                Ok(0)
            }
        }
        SendCase::MayOverTime => Ok(0),
        _ => Ok(0),
    }
}
pub fn response_send(mut stream: &TcpStream, response: Response) -> Result<usize, ()> {
    let msg_serial = unsafe { serialize_any(&response) };
    match SEND_CASE {
        SendCase::Normal => real_send(stream, msg_serial),
        SendCase::MayLoss => {
            let mut rng = rand::thread_rng();
            if rng.gen_range(0..100) > LOSS_RATE {
                real_send(stream, msg_serial)
            } else {
                Ok(0)
            }
        }
        SendCase::MayOverTime => Ok(0),
        SendCase::MustLoss => Ok(0),
    }
}
pub fn real_send(mut stream: &TcpStream, data: &[u8]) -> Result<usize, ()> {
    match stream.write(data) {
        Ok(num) => Ok(num),
        Err(_) => Err(()),
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

pub fn read_response_n(mut stream: &TcpStream) -> Result<Response, ()> {
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
pub fn read_response_short_time(mut stream: &TcpStream) -> Result<Response, ()> {
    let mut data = [0 as u8; MAX_BUFF];
    match stream.read(&mut data) {
        Ok(_) => {
            let get_resp = unsafe { deserialize_response(&data) };
            if !get_resp.check_checksum() {
                println!("{} = {:?}", client_prefix("error get resp"), get_resp);
                return Err(());
            }
            println!("{} {:?}", client_prefix("checksum OK"), get_resp);
            Ok(get_resp)
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
