use alloc::{
    borrow::ToOwned,
    collections::{BTreeMap, BTreeSet},
    vec,
    vec::Vec,
};

use serde::{de::DeserializeOwned, Serialize};
use tracing::debug;

use crate::{
    error::Error,
    messages::Packet,
    transport::{Transport, MAX_DATA_BYTES},
    utils::sleep_millis,
    SlidingWindow,
};

const MAX_POLL_CQ_RETRY: u32 = 100;
const POLL_INTERVAL: u32 = 1;
const WINDOW_SIZE: usize = 64;

/// Session provides send/receive between server/client
/// Session should act like a stream. Users will read/write from this object by using `send_bytes` and `recv_bytes`.
/// User can also pass in a serializable structure to send, or deserializable structure to recv.
/// Session will handle reorder and package loss.
pub struct Session {
    transport: Transport,
    /// Session ID
    id: u64,
    /// the largest seq of all packets sent
    seq: u64,
    /// the largest ack of all packets received and acknowledged
    ack: u64,
    /// seq to packet
    recv_buffer: BTreeMap<u64, Packet>,
}

impl Session {
    // TODO: exchange ack and syn using tcp
    pub fn new(id: u64, transport: Transport) -> Self {
        Self {
            transport,
            id,
            seq: 0,
            ack: 0,
            recv_buffer: BTreeMap::new(),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    // will ensure all bytes are sent and acknowledged by the remote end
    pub fn send_bytes(&mut self, bytes: Vec<u8>) -> Result<(), Error> {
        debug!("sending {} bytes", bytes.len());

        let packets = self.make_packets(bytes); // all packets to be sent
        let mut waiting: BTreeSet<u64> = packets.iter().map(|packet| packet.seq()).collect(); // packets that are waiting to be acknowledged by the remote end
        let mut window = SlidingWindow::new(packets.as_slice(), WINDOW_SIZE); // current window

        loop {
            let packets: Vec<_> = window
                .get()
                .iter()
                .filter_map(|packet| waiting.contains(&packet.seq()).then(|| packet.to_owned()))
                .collect();
            assert!(!packets.is_empty());

            self.transport.send_burst(packets)?;

            let mut round = 0;
            while round < MAX_POLL_CQ_RETRY {
                sleep_millis(POLL_INTERVAL);

                // recv acks, if reieved packets are not ack, insert them to recv_buffer and send back acks
                let packets = self.transport.try_recv()?;
                let mut acks = vec![];
                for packet in packets {
                    if !packet.is_ack() {
                        acks.push(Packet::new_ack(packet.seq(), self.id));
                        self.insert_recv_buffer(packet);
                    } else {
                        waiting.remove(&packet.ack());
                    }
                }
                self.transport.send_burst(acks)?;

                // try to move the window
                let last_sent_seq = window.last().seq();
                while !waiting.contains(&window.first().seq()) {
                    window.slide();
                    if window.is_closed() {
                        return Ok(());
                    }
                }

                // if window has moved, send new packets to the remote end and reset waiting round
                if window.last().seq() > last_sent_seq {
                    round = 0; // reset round

                    let new_packets = window
                        .get()
                        .iter()
                        .filter_map(|packet| {
                            (packet.seq() > last_sent_seq).then(|| packet.to_owned())
                        })
                        .collect();
                    self.transport.send_burst(new_packets)?;
                } else {
                    round += 1;
                }
            }
        }
    }

    // will return as soon as some bytes are received(order is guaranteed)
    pub fn recv_bytes(&mut self) -> Result<Vec<u8>, Error> {
        loop {
            // check if there are bytes ready to be returned to users
            let mut ready_bytes = vec![];
            while let Some(packet) = self.recv_buffer.remove(&self.ack) {
                self.ack += 1;
                let mut data = packet.into_data();
                ready_bytes.append(&mut data);
            }
            if !ready_bytes.is_empty() {
                debug!("received {} bytes", ready_bytes.len());
                return Ok(ready_bytes);
            }

            let packets = self.transport.recv()?;

            // send back acks
            let mut acks = vec![];
            for packet in packets {
                assert_eq!(self.id(), packet.session_id());
                if packet.is_ack() {
                    continue;
                }

                // insert the packet to buffer and reply with ack
                let ack_num = packet.seq();
                acks.push(Packet::new_ack(packet.seq(), self.id));
                debug!("send ack {ack_num}");

                self.insert_recv_buffer(packet);
            }
            self.transport.send_burst(acks)?;
        }
    }

    pub fn send<T: Serialize + Clone>(&mut self, value: T) -> Result<(), Error> {
        debug!("start sending");

        // prepare data
        let size = bincode::serialized_size(&value)? as usize;
        let mut data = vec![0; 8 + size];
        data[0..8].copy_from_slice(&size.to_be_bytes());
        bincode::serialize_into(&mut data[8..], &value)?;

        self.send_bytes(data)?;

        debug!("send succeeded");
        Ok(())
    }

    pub fn recv<R: DeserializeOwned>(&mut self) -> Result<R, Error> {
        debug!("start receiving");

        let mut bytes = self.recv_bytes()?;
        let size = usize::from_be_bytes(bytes[0..8].try_into().unwrap());
        debug!("need to recv {} bytes", size);
        bytes.reserve(size);

        while bytes.len() < size + 8 {
            let mut new_bytes = self.recv_bytes()?;
            bytes.append(&mut new_bytes);
        }

        debug!("receive suceeded");
        Ok(bincode::deserialize(&bytes[8..(8 + size)])?)
    }

    fn make_packets(&mut self, bytes: Vec<u8>) -> Vec<Packet> {
        bytes
            .chunks(MAX_DATA_BYTES)
            .map(|chunk| {
                let packet = Packet::new(self.seq, self.id, chunk.to_vec());
                self.seq += 1;
                packet
            })
            .collect()
    }

    fn insert_recv_buffer(&mut self, packet: Packet) {
        assert!(!packet.is_ack());
        if packet.seq() >= self.ack {
            self.recv_buffer.insert(packet.seq(), packet);
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use crate::utils::tests::{new_random_data, new_two_transport};

    #[test]
    fn session_works() {
        let (tp1, tp2) = new_two_transport();
        let (mut s1, mut s2) = (Session::new(0, tp1), Session::new(0, tp2));

        let bytes1 = new_random_data(64);
        let bytes1_c = bytes1.clone();
        let bytes2 = new_random_data(64);
        let bytes2_c = bytes2.clone();

        let s1_handle = std::thread::spawn(move || {
            s1.send_bytes(bytes1.clone()).unwrap();
            assert_eq!(s1.recv::<Vec<u8>>().unwrap(), bytes2);
        });

        let s2_handle = std::thread::spawn(move || {
            assert_eq!(s2.recv_bytes().unwrap(), bytes1_c);
            s2.send(bytes2_c.clone()).unwrap();
        });

        s1_handle.join().unwrap();
        s2_handle.join().unwrap();
    }

    #[test]
    // test whether the data will be successfully divided into small packets and received correctly by the remote
    fn send_bytes_huge() {
        let (tp1, tp2) = new_two_transport();
        let (mut s1, mut s2) = (Session::new(0, tp1), Session::new(0, tp2));

        let huge_bytes = new_random_data(4 * 1024 * 1024); // 4MB
        let huge_bytes_c = huge_bytes.clone(); // 4MB

        let s1_handle = std::thread::spawn(move || {
            s1.send_bytes(huge_bytes).unwrap();
        });

        let s2_handle = std::thread::spawn(move || {
            let mut buffer = vec![];
            while buffer.len() != huge_bytes_c.len() {
                let mut bytes = s2.recv_bytes().unwrap();
                buffer.append(&mut bytes);
            }
            assert_eq!(buffer, huge_bytes_c);
        });

        s1_handle.join().unwrap();
        s2_handle.join().unwrap();
    }

    #[test]
    // test whether the data will be successfully divided into small packets and received correctly by the remote
    fn send_huge() {
        let (tp1, tp2) = new_two_transport();
        let (mut s1, mut s2) = (Session::new(0, tp1), Session::new(0, tp2));

        let huge_bytes = new_random_data(4 * 1024 * 1024); // 4MB
        let huge_bytes_c = huge_bytes.clone(); // 4MB

        let s1_handle = std::thread::spawn(move || {
            s1.send(huge_bytes).unwrap();
        });

        let s2_handle = std::thread::spawn(move || {
            assert_eq!(s2.recv::<Vec<u8>>().unwrap(), huge_bytes_c);
        });

        s1_handle.join().unwrap();
        s2_handle.join().unwrap();
    }

    #[test]
    // send 1000 small packets
    fn send_small_packets() {
        const N_PACKETS: usize = 1000;
        let (tp1, tp2) = new_two_transport();
        let (mut s1, mut s2) = (Session::new(0, tp1), Session::new(0, tp2));

        let bytes = new_random_data(64);

        let s1_handle = std::thread::spawn(move || {
            for _ in 0..N_PACKETS {
                s1.send_bytes(bytes.clone()).unwrap();
            }
        });

        let s2_handle = std::thread::spawn(move || {
            for _ in 0..N_PACKETS {
                s2.recv_bytes().unwrap();
            }
        });

        s1_handle.join().unwrap();
        s2_handle.join().unwrap();
    }
}
