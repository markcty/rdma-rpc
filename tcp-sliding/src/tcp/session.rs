use crate::config::FRAME_CONTENT_MAX_LEN;
use crate::tcp::client::SendCase;
use crate::tcp::types::{Message, MESSAGE_CONTENT_SIZE};
use std::io::{Read, Write};
use std::net::TcpStream;
pub struct Session {
    pub stream: TcpStream,
}

impl Session {
    pub fn new(server_addr: &str) -> Session {
        Session {
            stream: TcpStream::connect(server_addr).expect("connect failed"),
        }
    }
    pub fn send(&self, data: &[u8]) {
        let msg_total_num = (data.len() + MESSAGE_CONTENT_SIZE - 1) / MESSAGE_CONTENT_SIZE;
        let test_msg = Message {
            id: 1,
            content: data[0..MESSAGE_CONTENT_SIZE]
                .try_into()
                .expect("slice with incorrect length"),
        };
        println!("assemble msg = {:?}", test_msg);
    }
    fn slice_send(mut self, context: &[u8], mode: &SendCase) {
        match mode {
            SendCase::Normal => {
                self.stream.write(context).unwrap();
            }
            SendCase::MayLoss => {}
            SendCase::MayOverTime => {}
        };
    }
}
