use alloc::vec;
use alloc::vec::Vec;

use crate::{error::Error, messages::Packet, transport::Transport};
use serde::{de::DeserializeOwned, Serialize};
use tracing::{info, warn};

// for the MR, its layout is:
// |0    ... 4096 | // send buffer
// |4096 ... 8192 | // receive buffer
const MESSAGE_CONTENT_SIZE: usize = 1;
const ROUND_MAX: u32 = 500;
const INTERVAL_PER_ROUND: u32 = 10;
const INTERVAL_ONE_WINDOW: u32 = 2;
const WINDOW_SIZE: usize = 2;
/// Session provides send/receive between server/client
/// Session should act like a stream. Users will read/write from this object.
/// Should handle reorder and package loss.
pub struct Session {
    transport: Transport,
    id: u64,
    syn: u64,
    ack: u64,
}

#[derive(Debug)]
pub enum WindowStatus {
    SendFinished,
    RecvFinished,
    CurrentWindowDone,
    Continue,
}

#[derive(Debug)]
pub struct Window {
    total_packet_num: u64,
    seq_num_upper_bound: u64,
    window_base: usize,
    window_upper: usize,
    waiting_range: [bool; WINDOW_SIZE],
    cur_waiting_num: usize,
}

impl Window {
    pub fn new(init_num: usize, packet_num: u64) -> Self {
        let upper = if WINDOW_SIZE > packet_num as usize {
            packet_num as usize
        } else {
            WINDOW_SIZE
        };
        Self {
            total_packet_num: packet_num,
            seq_num_upper_bound: init_num as u64 + packet_num,
            window_base: init_num,
            window_upper: init_num + upper,
            waiting_range: [true; WINDOW_SIZE],
            cur_waiting_num: upper,
        }
    }
    pub fn send_packets(
        &self,
        data: &Vec<u8>,
        transport: &mut Transport,
        session_id: u64,
    ) -> Result<(), Error> {
        for seq_num in self.window_base..self.window_upper {
            info!("waiting range = {:?}", self.waiting_range);
            // only re-send those still waiting
            if self.waiting_range[seq_num - self.window_base] {
                sleep_millis(INTERVAL_ONE_WINDOW);
                let down_bound = seq_num as usize * MESSAGE_CONTENT_SIZE;
                let up_bound = down_bound as usize + MESSAGE_CONTENT_SIZE;
                let cur_packet = Packet::new(
                    0,
                    seq_num as u64,
                    session_id,
                    data[down_bound..up_bound].try_into().unwrap(),
                    self.total_packet_num,
                );
                info!("send packet {:?}", &cur_packet);
                let packets = vec![cur_packet];
                transport.send_all(packets)?;
            }
        }
        Ok(())
    }
    pub fn sender_recv(&mut self, transport: &mut Transport) -> Result<WindowStatus, Error> {
        let packets = transport.try_recv()?;
        if packets.len() != 0 {
            info!("packets len = {:?}", packets.len());
            for packet in packets {
                // get one packet, check the ack number
                let ack_num = packet.ack();
                if self.window_base <= ack_num as usize
                    && (ack_num as usize) < self.window_upper
                    && self.waiting_range[ack_num as usize - self.window_base]
                {
                    info!("recv ack {}", packet.ack());
                    self.cur_waiting_num -= 1;
                    self.waiting_range[ack_num as usize - self.window_base] = false;
                    if self.cur_waiting_num == 0 {
                        if self.window_upper == self.seq_num_upper_bound as usize {
                            // all packets are done
                            info!("sending ended");
                            return Ok(WindowStatus::SendFinished);
                        } else {
                            // this window's packets are done
                            // reset for the next window
                            self.reset_next_window();
                            return Ok(WindowStatus::CurrentWindowDone);
                        }
                    }
                } else {
                    let seq_num = packet.seq();
                    if seq_num >= self.seq_num_upper_bound && ack_num == 0 {
                        // server start to resend resp data
                        return Ok(WindowStatus::SendFinished);
                    }
                }
            }
        }
        Ok(WindowStatus::Continue)
    }
    fn reset_next_window(&mut self) {
        self.window_base = self.window_upper;
        self.window_upper = if self.window_base + WINDOW_SIZE < self.seq_num_upper_bound as usize {
            self.window_base + WINDOW_SIZE
        } else {
            self.seq_num_upper_bound as usize
        };
        self.cur_waiting_num = self.window_upper - self.window_base;
        self.waiting_range = [true; WINDOW_SIZE];
        info!("goto the next window");
    }
}
impl Session {
    // TODO: exchange ack and syn using tcp
    pub fn new(id: u64, transport: Transport) -> Self {
        Self {
            transport,
            id,
            ack: 0,
            syn: 0,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }
    pub(crate) fn send<T: Serialize + Clone>(&mut self, data: T) -> Result<(), Error> {
        let data = bincode::serialize(&data)?;
        let mut round_cnt;
        let packet_total_num =
            ((data.len() + MESSAGE_CONTENT_SIZE - 1) / MESSAGE_CONTENT_SIZE) as u64;
        let mut send_window = Window::new(self.syn as usize, packet_total_num);
        info!("get the window {:?}", send_window);
        // wait until the packet is received by the remote
        info!("starg sending, [packet num = {:?}]", packet_total_num);
        'send: loop {
            round_cnt = 0;
            send_window.send_packets(&data, &mut self.transport, self.id)?;

            'listen: loop {
                //TODO: recv multi packets
                // listen loop
                if round_cnt >= ROUND_MAX {
                    // listening over time
                    break;
                }
                sleep_millis(INTERVAL_PER_ROUND);
                match send_window.sender_recv(&mut self.transport) {
                    Ok(WindowStatus::CurrentWindowDone) => break 'listen,
                    Ok(WindowStatus::SendFinished) => {
                        info!("sender finished");
                        break 'send;
                    }
                    _ => {}
                };
                round_cnt += 1;
            }
        }
        Ok(())
    }

    /// Recv the next request
    pub(crate) fn recv<R: DeserializeOwned>(&mut self) -> Result<R, Error> {
        info!("start recv waiting");
        let mut buffer = Vec::new();
        let mut window_base: usize = 0;
        let mut window_upper: usize = window_base + WINDOW_SIZE;
        let mut accept_range: [bool; WINDOW_SIZE] = [true; WINDOW_SIZE];
        let mut accept_left = WINDOW_SIZE;
        let mut total_packet_num: u64 = 0;
        let mut recevied_packet_num: u64 = 0;
        buffer.resize(buffer.len() + WINDOW_SIZE * MESSAGE_CONTENT_SIZE, 0u8);
        'listen: loop {
            // TODO: assemble the packets to R
            let packets = self.transport.recv()?;
            for packet in packets {
                assert_eq!(packet.session_id(), self.id);
                if total_packet_num == 0 {
                    total_packet_num = packet.total_num();
                }
                info!("get packet {:?}", &packet);
                let syn_num = packet.seq();
                if window_base as u64 <= syn_num
                    && syn_num < window_upper as u64
                    && accept_range[(syn_num - window_base as u64) as usize]
                {
                    recevied_packet_num += 1;
                    accept_left -= 1;
                    accept_range[(syn_num - window_base as u64) as usize] = false;
                    insert_buffer(
                        &mut buffer,
                        packet.data(),
                        (syn_num) as usize * MESSAGE_CONTENT_SIZE,
                    );
                    if recevied_packet_num == total_packet_num {
                        let resp = Packet::new_empty(syn_num, 0, self.id);
                        info!("reply with {:?}", resp);
                        let resps = vec![resp];
                        self.transport.send_all(resps)?;
                        break 'listen;
                    }
                    if accept_left == 0 {
                        window_base += WINDOW_SIZE;
                        window_upper = window_base + WINDOW_SIZE;
                        accept_range = [true; WINDOW_SIZE];
                        accept_left = WINDOW_SIZE;
                        buffer.resize(buffer.len() + WINDOW_SIZE * MESSAGE_CONTENT_SIZE, 0u8);
                    }
                }
                let resp = Packet::new_empty(syn_num, 0, self.id);
                info!("reply with {:?}", resp);
                let resps = vec![resp];
                self.transport.send_all(resps)?;
            }
        }
        Ok(bincode::deserialize(&buffer)?)
    }
}

fn insert_buffer(data: &mut [u8], inner: &[u8], base: usize) {
    let len = inner.len();
    for i in 0..len {
        data[i + base] = inner[i];
    }
}
fn sleep_millis(duration: u32) {
    unsafe {
        libc::usleep(1000 * duration);
    }
}
