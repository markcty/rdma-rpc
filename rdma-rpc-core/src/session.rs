use alloc::borrow::ToOwned;
use alloc::vec::Vec;
use alloc::{collections::BTreeSet, vec};

use crate::utils::sleep_millis;
use crate::{
    error::Error,
    messages::Packet,
    transport::{Transport, MAX_PACKET_BYTES},
};
use serde::{de::DeserializeOwned, Serialize};
use tracing::debug;

const MAX_POLL_CQ_RETRY: u32 = 500;
const POLL_INTERVAL: u32 = 1;
const WINDOW_SIZE: usize = 2;
const MAX_RETRY: u32 = 10;

/// Session provides send/receive between server/client
/// Session should act like a stream. Users will read/write from this object.
/// Should handle reorder and package loss.
pub struct Session {
    transport: Transport,
    id: u64,
    seq: u64,
}

impl Session {
    // TODO: exchange ack and syn using tcp
    pub fn new(id: u64, transport: Transport) -> Self {
        Self {
            transport,
            id,
            seq: 0,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub(crate) fn send<T: Serialize + Clone>(&mut self, data: T) -> Result<(), Error> {
        debug!("start sending");

        let data = bincode::serialize(&data)?;
        let packets = self.make_packets(data);

        // sliding window: selective repeat
        for window in packets.chunks(WINDOW_SIZE) {
            self.send_window_checked(window)?;
        }

        debug!("send succeeded");
        Ok(())
    }

    fn make_packets(&mut self, data: Vec<u8>) -> Vec<Packet> {
        let chunks = data.chunks(MAX_PACKET_BYTES);
        let total_num = chunks.len();
        chunks
            .map(|chunk| {
                let packet = Packet::new(self.seq, self.id, chunk.to_vec(), total_num as u64);
                self.seq += 1;
                packet
            })
            .collect()
    }

    // will send current window until succeeded
    fn send_window_checked(&mut self, window: &[Packet]) -> Result<(), Error> {
        let mut waiting = vec![true; window.len()]; // whether a packet has been sent and acknowledged by the remote end
        let first_seq = window[0].seq();
        let last_seq = window.last().unwrap().seq();
        for _ in 0..MAX_RETRY {
            // only re-send those still waiting
            let packets: Vec<Packet> = waiting
                .iter()
                .enumerate()
                .filter_map(|(i, waiting)| waiting.then(|| window[i].to_owned()))
                .collect();

            self.transport.send_burst(packets)?;

            // receive acks
            for _ in 0..MAX_POLL_CQ_RETRY {
                sleep_millis(POLL_INTERVAL);
                let packet = if let Some(packet) = self.transport.try_recv()? {
                    packet
                } else {
                    continue;
                };

                if !packet.is_ack() || packet.ack() > last_seq {
                    return Ok(()); // the remote end has got all packets and entered send state
                }

                if first_seq <= packet.ack() && packet.ack() <= last_seq {
                    waiting[(packet.ack() - first_seq) as usize] = false;
                }

                if waiting.iter().all(|waiting| !(*waiting)) {
                    return Ok(());
                }
            }
            debug!("send window again");
        }

        Err(Error::Timeout)
    }

    pub(crate) fn recv<R: DeserializeOwned>(&mut self) -> Result<R, Error> {
        debug!("start receiving");
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
                let ack = Packet::new_ack(ack_num, self.id);
                self.transport.send_burst(vec![ack])?;
                debug!("send ack {ack_num}");

                if buffer.len() == total_packet_num as usize {
                    let bytes: Vec<u8> = buffer
                        .into_iter()
                        .flat_map(|packet| packet.data().to_vec())
                        .collect();
                    debug!("receive suceeded");
                    return Ok(bincode::deserialize(bytes.as_slice())?);
                }
            }
        }
    }
}
