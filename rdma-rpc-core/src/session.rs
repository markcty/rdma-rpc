use alloc::collections::BTreeSet;
use alloc::vec::Vec;
use alloc::{borrow::ToOwned, vec};

use crate::transport::MAX_PACKET_BYTES;
use crate::{error::Error, messages::Packet, transport::Transport};
use serde::{de::DeserializeOwned, Serialize};
use tracing::{debug, info, warn};

// for the MR, its layout is:
// |0    ... 4096 | // send buffer
// |4096 ... 8192 | // receive buffer
const MESSAGE_CONTENT_SIZE: usize = 1;
/// Max number of rounds for trying to recv the ack
const MAX_WAIT_ROUND: u32 = 500;
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

// #[derive(Debug)]
// pub struct Window {
//     total_packet_num: u64,
//     seq_num_upper_bound: u64,
//     window_base: usize,
//     window_upper: usize,
//     waiting_range: [bool; WINDOW_SIZE],
//     cur_waiting_num: usize,
// }

// impl Window {
//     pub fn new_recv_window(init_num: usize) -> Self {
//         Self {
//             total_packet_num: 0,
//             seq_num_upper_bound: 0,
//             window_base: init_num,
//             window_upper: init_num,
//             waiting_range: [true; WINDOW_SIZE],
//             cur_waiting_num: 0,
//         }
//     }
//     fn set_packet_num(&mut self, packet_num: u64) {
//         let upper = if WINDOW_SIZE > packet_num as usize {
//             packet_num as usize
//         } else {
//             WINDOW_SIZE
//         };
//         self.total_packet_num = packet_num;
//         self.seq_num_upper_bound = self.window_base as u64 + packet_num;
//         self.window_upper = self.window_base + upper;
//         self.cur_waiting_num = upper;
//     }

//     pub fn new_sender_window(init_num: usize, packet_num: u64) -> Self {
//         let upper = if WINDOW_SIZE > packet_num as usize {
//             packet_num as usize
//         } else {
//             WINDOW_SIZE
//         };
//         Self {
//             total_packet_num: packet_num,
//             seq_num_upper_bound: init_num as u64 + packet_num,
//             window_base: init_num,
//             window_upper: init_num + upper,
//             waiting_range: [true; WINDOW_SIZE],
//             cur_waiting_num: upper,
//         }
//     }
//     pub fn send_packets(
//         &self,
//         data: &Vec<u8>,
//         transport: &mut Transport,
//         session_id: u64,
//     ) -> Result<(), Error> {
//         for seq_num in self.window_base..self.window_upper {
//             info!("waiting range = {:?}", self.waiting_range);
//             // only re-send those still waiting
//             if self.waiting_range[seq_num - self.window_base] {
//                 sleep_millis(INTERVAL_ONE_WINDOW);
//                 let down_bound = seq_num as usize * MESSAGE_CONTENT_SIZE;
//                 let up_bound = down_bound as usize + MESSAGE_CONTENT_SIZE;
//                 let cur_packet = Packet::new(
//                     0,
//                     seq_num as u64,
//                     session_id,
//                     data[down_bound..up_bound].try_into().unwrap(),
//                     self.total_packet_num,
//                 );
//                 info!("send packet {:?}", &cur_packet);
//                 let packets = vec![cur_packet];
//                 transport.send_burst(packets)?;
//             }
//         }
//         Ok(())
//     }
//     pub fn sender_recv(&mut self, transport: &mut Transport) -> Result<WindowStatus, Error> {
//         let packets = transport.try_recv()?;
//         if packets.len() != 0 {
//             info!("packets len = {:?}", packets.len());
//             for packet in packets {
//                 // get one packet, check the ack number
//                 let ack_num = packet.ack();
//                 if self.window_base <= ack_num as usize
//                     && (ack_num as usize) < self.window_upper
//                     && self.waiting_range[ack_num as usize - self.window_base]
//                 {
//                     info!("recv ack {}", packet.ack());
//                     self.cur_waiting_num -= 1;
//                     self.waiting_range[ack_num as usize - self.window_base] = false;
//                     if self.cur_waiting_num == 0 {
//                         if self.window_upper == self.seq_num_upper_bound as usize {
//                             // all packets are done
//                             info!("sending ended");
//                             return Ok(WindowStatus::SendFinished);
//                         } else {
//                             // this window's packets are done
//                             // reset for the next window
//                             self.reset_next_window();
//                             return Ok(WindowStatus::CurrentWindowDone);
//                         }
//                     }
//                 } else {
//                     let seq_num = packet.seq();
//                     if seq_num >= self.seq_num_upper_bound && ack_num == 0 {
//                         // server start to resend resp data
//                         return Ok(WindowStatus::SendFinished);
//                     }
//                 }
//             }
//         }
//         Ok(WindowStatus::Continue)
//     }
//     // pub fn receiver_recv(
//     //     &mut self,
//     //     transport: &mut Transport,
//     //     session_id: u64,
//     //     buffer: &mut Vec<u8>,
//     // ) -> Result<WindowStatus, Error> {
//     //     // TODO: assemble the packets to R
//     //     let packets = transport.recv()?;
//     //     for packet in packets {
//     //         assert_eq!(packet.session_id(), session_id);
//     //         if self.total_packet_num == 0 {
//     //             self.set_packet_num(packet.total_num());
//     //             warn!("setup recv window {:?}", self);
//     //         }
//     //         let seq_num = packet.seq();
//     //         // reply with ack = seq
//     //         let resp = Packet::new_empty(seq_num, 0, session_id);
//     //         info!("reply with {:?}", resp);
//     //         let resps = vec![resp];
//     //         transport.send_burst(resps)?;

//     //         if self.window_base as u64 <= seq_num
//     //             && seq_num < self.window_upper as u64
//     //             && self.waiting_range[(seq_num - self.window_base as u64) as usize]
//     //         {
//     //             info!("get packet {:?}", &packet);
//     //             // this packet is acceptable
//     //             self.cur_waiting_num -= 1;
//     //             self.waiting_range[(seq_num - self.window_base as u64) as usize] = false;
//     //             insert_buffer(
//     //                 buffer,
//     //                 packet.data(),
//     //                 (seq_num) as usize * MESSAGE_CONTENT_SIZE,
//     //             );
//     //             if self.cur_waiting_num == 0 {
//     //                 if self.window_upper == self.seq_num_upper_bound as usize {
//     //                     warn!("receiver recv ended");
//     //                     return Ok(WindowStatus::RecvFinished);
//     //                 } else {
//     //                     self.reset_next_window();
//     //                     buffer.resize(buffer.len() + WINDOW_SIZE * MESSAGE_CONTENT_SIZE, 0u8);
//     //                     return Ok(WindowStatus::CurrentWindowDone);
//     //                 }
//     //             }
//     //         }
//     //     }
//     //     Ok(WindowStatus::Continue)
//     // }

//     fn reset_next_window(&mut self) {
//         self.window_base = self.window_upper;
//         self.window_upper = if self.window_base + WINDOW_SIZE < self.seq_num_upper_bound as usize {
//             self.window_base + WINDOW_SIZE
//         } else {
//             self.seq_num_upper_bound as usize
//         };
//         self.cur_waiting_num = self.window_upper - self.window_base;
//         self.waiting_range = [true; WINDOW_SIZE];
//         info!("goto the next window");
//     }
// }

impl Session {
    // TODO: exchange ack and syn using tcp
    pub fn new(id: u64, transport: Transport) -> Self {
        Self {
            transport,
            id,
            syn: 0,
            ack: 0,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub(crate) fn send<T: Serialize + Clone>(&mut self, data: T) -> Result<(), Error> {
        let data = bincode::serialize(&data)?;
        let packets = self.make_packets(data);

        // sliding window
        for window in packets.chunks(WINDOW_SIZE) {
            self.send_window_checked(window)?;
        }

        Ok(())
    }

    fn make_packets(&mut self, data: Vec<u8>) -> Vec<Packet> {
        let chunks = data.chunks(MAX_PACKET_BYTES);
        let total_num = chunks.len();
        chunks
            .map(|chunk| {
                let packet = Packet::new(
                    self.ack,
                    self.syn,
                    self.id,
                    chunk.to_vec(),
                    total_num as u64,
                );
                self.syn += 1;
                packet
            })
            .collect()
    }

    // will send current window until succeeded
    fn send_window_checked(&mut self, window: &[Packet]) -> Result<(), Error> {
        let mut waiting = vec![true; window.len()]; // whether a packet has been sent and acknowledged by the remote end
        let first_seq = window[0].seq();
        let last_seq = window.last().unwrap().seq();
        loop {
            // only re-send those still waiting
            let packets: Vec<Packet> = waiting
                .iter()
                .enumerate()
                .filter_map(|(i, waiting)| waiting.then(|| window[i].to_owned()))
                .collect();

            self.transport.send_burst(packets)?;

            // receive acks
            for _ in 0..MAX_WAIT_ROUND {
                sleep_millis(INTERVAL_PER_ROUND);
                let packets = self.transport.try_recv()?;
                for packet in packets {
                    if !packet.is_ack() || packet.ack() > last_seq {
                        return Ok(()); // the remote end has got all packets and entered send state
                    }
                    if first_seq <= packet.ack() && packet.ack() <= last_seq {
                        waiting[(packet.ack() - first_seq) as usize] = false;
                    }
                }
                if waiting.iter().all(|sent| *sent) {
                    return Ok(());
                }
            }
        }
    }

    pub(crate) fn recv<R: DeserializeOwned>(&mut self) -> Result<R, Error> {
        let mut buffer = BTreeSet::new();
        loop {
            let packets = self.transport.recv()?;
            let mut total_packet_num = 0;
            for packet in packets {
                assert_eq!(self.id(), packet.session_id());
                if total_packet_num == 0 {
                    total_packet_num = packet.total_num();
                    debug!("sender has {total_packet_num} packets to send");
                }

                // insert the packet to buffer and reply with ack
                let ack_num = packet.seq();
                buffer.insert(packet);
                let ack = Packet::new_ack(ack_num, self.syn, self.id);
                self.transport.send_burst(vec![ack])?;
                debug!("send ack {ack_num}");

                if buffer.len() == total_packet_num as usize {
                    let bytes: Vec<u8> = buffer
                        .into_iter()
                        .flat_map(|packet| packet.data().to_vec())
                        .collect();
                    return Ok(bincode::deserialize(bytes.as_slice())?);
                }
            }
        }
    }
}

fn sleep_millis(duration: u32) {
    unsafe {
        libc::usleep(1000 * duration);
    }
}
