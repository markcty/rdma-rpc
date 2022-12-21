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
    // fn send_num(&mut self,total_num:u32){

    // }
    pub(crate) fn send<T: Serialize + Clone>(&mut self, data: T) -> Result<(), Error> {
        let data = bincode::serialize(&data)?;

        let mut round_cnt;
        let packet_total_num =
            ((data.len() + MESSAGE_CONTENT_SIZE - 1) / MESSAGE_CONTENT_SIZE) as u64;
        let mut waiting_range: [bool; WINDOW_SIZE] = [true; WINDOW_SIZE];
        let mut window_base: usize = 0;
        let mut window_upper = if WINDOW_SIZE > packet_total_num as usize {
            packet_total_num as usize
        } else {
            WINDOW_SIZE
        };
        let mut waiting_num: usize = window_upper;
        // wait until the packet is received by the remote
        info!("starg sending, [packet num = {:?}]", packet_total_num);
        'send: loop {
            round_cnt = 0;
            for seq_num in window_base..window_upper {
                info!("waiting range = {:?}", waiting_range);
                // only re-send those still waiting
                if waiting_range[seq_num - window_base] {
                    //TODO: set the first packet for sending packet len
                    sleep_millis(INTERVAL_ONE_WINDOW);
                    let down_bound = seq_num as usize * MESSAGE_CONTENT_SIZE;
                    let up_bound = down_bound as usize + MESSAGE_CONTENT_SIZE;
                    let cur_packet = Packet::new(
                        0,
                        seq_num as u64,
                        self.id,
                        data[down_bound..up_bound].try_into().unwrap(),
                    );
                    info!("send packet {:?}", &cur_packet);
                    let packets = vec![cur_packet];
                    self.transport.send_all(packets)?;
                }
            }

            'listen: loop {
                //TODO: recv multi packets
                // listen loop
                if round_cnt >= ROUND_MAX {
                    // listening over time
                    break;
                }
                sleep_millis(INTERVAL_PER_ROUND);
                let packets = self.transport.try_recv()?;
                if packets.len() != 0 {
                    info!("packets len = {:?}", packets.len());
                    for packet in packets {
                        // get one packet, check the ack number
                        let ack_num = packet.ack();
                        if window_base <= ack_num as usize
                            && (ack_num as usize) < window_upper
                            && waiting_range[ack_num as usize - window_base]
                        {
                            info!("recv ack {}", packet.ack());
                            waiting_num -= 1;
                            waiting_range[ack_num as usize - window_base] = false;
                            if waiting_num == 0 {
                                if window_upper == packet_total_num as usize {
                                    // all packets are done
                                    info!("sending ended");
                                    break 'send;
                                } else {
                                    // this window's packets are done
                                    // reset for the next window

                                    window_base = window_upper;
                                    window_upper =
                                        if window_base + WINDOW_SIZE < packet_total_num as usize {
                                            window_base + WINDOW_SIZE
                                        } else {
                                            packet_total_num as usize
                                        };
                                    waiting_num = window_upper - window_base;
                                    waiting_range = [true; WINDOW_SIZE];
                                    warn!("goto the next window");
                                    break 'listen;
                                }
                            }
                        }
                    }
                }
                round_cnt += 1;
            }
        }

        // in the end, send the FIN packet to server
        let fin_packet = Packet::new_fin(self.ack, self.syn, self.id);
        info!("send FIN send packet: {:?}", fin_packet);
        let packets = vec![fin_packet];
        self.transport.send_all(packets)?;
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
        buffer.resize(buffer.len() + WINDOW_SIZE * MESSAGE_CONTENT_SIZE, 0u8);
        loop {
            // TODO: assemble the packets to R
            let packets = self.transport.recv()?;
            for packet in packets {
                assert_eq!(packet.session_id(), self.id);

                info!("get packet {:?}", &packet);
                if packet.fin() {
                    return Ok(bincode::deserialize(&buffer)?);
                }
                let syn_num = packet.seq();
                if window_base as u64 <= syn_num
                    && syn_num < window_upper as u64
                    && accept_range[(syn_num - window_base as u64) as usize]
                {
                    accept_left -= 1;
                    accept_range[(syn_num - window_base as u64) as usize] = false;
                    insert_buffer(
                        &mut buffer,
                        packet.data(),
                        (syn_num) as usize * MESSAGE_CONTENT_SIZE,
                    );
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
