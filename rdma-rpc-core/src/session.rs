use alloc::{format, sync::Arc};

use crate::{error::Error, messages::Packet, transport::Transport};
use bytes::{BufMut, BytesMut};
use serde::{de::DeserializeOwned, Serialize};
use tracing::info;
use KRdmaKit::{context::Context, MemoryRegion};

// for the MR, its layout is:
// |0    ... 4096 | // send buffer
// |4096 ... 8192 | // receive buffer
#[allow(unused)]
const DATA_SIZE: usize = 4096;
#[allow(unused)]
const SESSION_START: usize = 2048;
#[allow(unused)]
const PAKET_HEADER: usize = 256;
const MESSAGE_CONTENT_SIZE: usize = 1;
const ROUND_MAX: u32 = 500;
const TIME_PER_ROUND: u32 = 10;
const WINDOW_GAP_TIME: u32 = 2;
const WINDOW_SIZE: usize = 2;
/// Session provides send/receive between server/client
/// Session should act like a stream. Users will read/write from this object.
/// Should handle reorder and package loss.
pub struct Session {
    transport: Transport,
    id: u64,
    pub mr: MemoryRegion,
    syn: u64,
    ack: u64,
}

#[allow(unused)]
impl Session {
    // TODO: exchange ack and syn using tcp
    pub fn new(context: Arc<Context>, id: u64, transport: Transport) -> Self {
        // create mr
        let mr = MemoryRegion::new(Arc::clone(&context), DATA_SIZE as usize)
            .map_err(|err| Error::Internal(format!("failed to allocate MR, {err}")))
            .unwrap();

        Self {
            transport,
            id,
            mr,
            ack: 0,
            syn: 0,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }
    fn send_start(&mut self) -> Result<(), Error> {
        let empty_packet = Packet::new_empty(self.ack, self.syn, self.id);
        let mut round_cnt = ROUND_MAX;
        loop {
            if round_cnt >= ROUND_MAX {
                info!("send end packet");
                self.transport.send(empty_packet.clone())?;
                round_cnt = 0;
            }
            sleep_millis(TIME_PER_ROUND);

            if let Some(packet) = self.transport.try_recv()? {
                if packet.ack() == self.syn + 1 {
                    info!("recv ack {}", packet.ack());
                    self.syn += 1;
                    break;
                }
            }
            round_cnt += 1;
        }
        Ok(())
    }
    fn recv_start(&mut self) -> Result<(), Error> {
        let empty_packet = Packet::new_empty(self.ack, self.syn, self.id);
        let mut round_cnt = ROUND_MAX;
        loop {
            if round_cnt >= ROUND_MAX {
                info!("send end packet");
                self.transport.send(empty_packet.clone())?;
                round_cnt = 0;
            }
            sleep_millis(TIME_PER_ROUND);

            if let Some(packet) = self.transport.try_recv()? {
                if packet.ack() == self.syn + 1 {
                    info!("recv ack {}", packet.ack());
                    self.syn += 1;
                    break;
                }
            }
            round_cnt += 1;
        }
        Ok(())
    }

    fn send_end(&mut self) -> Result<(), Error> {
        let empty_end_packet = Packet::new_empty(self.ack, self.syn, self.id);
        let mut round_cnt = ROUND_MAX;
        loop {
            if round_cnt >= ROUND_MAX {
                info!("send end packet");
                self.transport.send(empty_end_packet.clone())?;
                round_cnt = 0;
            }
            sleep_millis(TIME_PER_ROUND);

            if let Some(packet) = self.transport.try_recv()? {
                if packet.ack() == self.syn + 1 {
                    info!("recv ack {}", packet.ack());
                    self.syn += 1;
                    break;
                }
            }
            round_cnt += 1;
        }
        self.transport
            .send(Packet::new_empty(self.ack, self.syn, self.id))?;
        Ok(())
    }

    fn recv_end(&mut self) -> Result<(), Error> {
        let empty_end_packet = Packet::new_empty(self.ack, self.syn, self.id);
        let mut round_cnt = ROUND_MAX;
        loop {
            if round_cnt >= ROUND_MAX {
                info!("send end packet");
                self.transport.send(empty_end_packet.clone())?;
                round_cnt = 0;
            }
            sleep_millis(TIME_PER_ROUND);

            if let Some(packet) = self.transport.try_recv()? {
                if packet.ack() == self.syn + 1 {
                    info!("recv ack {}", packet.ack());
                    self.syn += 1;
                    break;
                }
            }
            round_cnt += 1;
        }
        Ok(())
    }
    pub(crate) fn send<T: Serialize + Clone>(&mut self, data: T) -> Result<(), Error> {
        // TODO: devide data into multiple packets if needed
        let data = bincode::serialize(&data)?;
        let mut base = 0;
        let mut upper = if MESSAGE_CONTENT_SIZE > data.len() {
            data.len()
        } else {
            MESSAGE_CONTENT_SIZE
        };
        let mut round_cnt = ROUND_MAX;
        let packet_total_num =
            ((data.len() + MESSAGE_CONTENT_SIZE - 1) / MESSAGE_CONTENT_SIZE) as u64;
        let mut send_packet = Packet::new(
            self.ack,
            self.syn,
            self.id,
            data.clone()[base..upper].try_into().unwrap(),
        );
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
        loop {
            if round_cnt >= ROUND_MAX {
                // over time
                let mut send_flag = false;
                for seq_num in window_base..window_upper {
                    info!("waiting range = {:?}", waiting_range);
                    // only re-send those still waiting
                    if waiting_range[seq_num - window_base] {
                        send_flag = true;
                        sleep_millis(WINDOW_GAP_TIME);
                        let down_bound = seq_num as usize * MESSAGE_CONTENT_SIZE;
                        let up_bound = down_bound as usize + MESSAGE_CONTENT_SIZE;
                        let cur_packet = Packet::new(
                            0,
                            seq_num as u64,
                            self.id,
                            data[down_bound..up_bound].try_into().unwrap(),
                        );
                        info!("send packet {:?}", &cur_packet);

                        self.transport.send(cur_packet)?;
                    }
                }
                if send_flag == false {
                    window_base = window_upper;
                    window_upper = if window_base + WINDOW_SIZE < packet_total_num as usize {
                        window_base + WINDOW_SIZE
                    } else {
                        packet_total_num as usize
                    };
                    waiting_num = window_upper - window_base;
                    waiting_range = [true; WINDOW_SIZE];
                    round_cnt = ROUND_MAX;
                    continue;
                }
            }
            sleep_millis(TIME_PER_ROUND);

            if let Some(packet) = self.transport.try_recv()? {
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
                            info!("sending ended");
                            break;
                        } else {
                            window_base = window_upper;
                            window_upper = if window_base + WINDOW_SIZE < packet_total_num as usize
                            {
                                window_base + WINDOW_SIZE
                            } else {
                                packet_total_num as usize
                            };
                            waiting_num = window_upper - window_base;
                            waiting_range = [true; WINDOW_SIZE];
                            round_cnt = ROUND_MAX;
                        }
                    }
                }
            }
            round_cnt += 1;
        }
        let fin_packet = Packet::new_fin(self.ack, self.syn, self.id);
        self.transport.send(fin_packet.clone())?;
        info!("send FIN send packet: {:?}", fin_packet);
        Ok(())
    }
    #[allow(unused)]
    pub(crate) fn send_v0<T: Serialize + Clone>(&mut self, data: T) -> Result<(), Error> {
        // TODO: devide data into multiple packets if needed
        let packet = Packet::new(self.ack, self.syn, self.id, bincode::serialize(&data)?);
        self.transport.send(packet.clone())?;

        // wait until the packet is received by the remote
        loop {
            sleep_millis(10);
            if let Some(packet) = self.transport.try_recv()? {
                if packet.ack() == self.syn + 1 {
                    info!("recv ack {}", packet.ack());
                    self.syn += 1;
                    break Ok(());
                }
            }

            // resend
            self.transport.send(packet.clone())?;
        }
    }

    /// Recv the next request
    pub(crate) fn recv<R: DeserializeOwned>(&mut self) -> Result<R, Error> {
        info!("start recv waiting");
        let mut buffer = BytesMut::with_capacity(DATA_SIZE);
        let mut window_base: usize = 0;
        let mut window_upper: usize = window_base + WINDOW_SIZE;
        let mut accept_range: [bool; WINDOW_SIZE] = [true; WINDOW_SIZE];
        let mut cur_buffer = [0u8; WINDOW_SIZE * MESSAGE_CONTENT_SIZE];
        let mut accept_left = WINDOW_SIZE;
        // cur_buffer[0] = 8u8;
        loop {
            // TODO: assemble the packets to R
            let packet = self.transport.recv()?;
            assert_eq!(packet.session_id(), self.id);
            info!("get packet {:?}", &packet);
            if packet.fin == 1 {
                return Ok(bincode::deserialize(&buffer)?);
            }
            let syn_num = packet.seq();
            if window_base as u64 <= syn_num
                && syn_num < window_upper as u64
                && accept_range[(syn_num - window_base as u64) as usize]
            {
                accept_left -= 1;
                accept_range[(syn_num - window_base as u64) as usize] = false;
                assemble_cur_buffer(
                    &mut cur_buffer,
                    packet.data(),
                    (syn_num - window_base as u64) as usize * MESSAGE_CONTENT_SIZE,
                );
                if accept_left == 0 {
                    buffer.put(&cur_buffer[..]);
                    window_base += WINDOW_SIZE;
                    window_upper = window_base + WINDOW_SIZE;
                    accept_range = [true; WINDOW_SIZE];
                    accept_left = WINDOW_SIZE;
                    cur_buffer = [0u8; WINDOW_SIZE * MESSAGE_CONTENT_SIZE];
                }
            }
            let resp = Packet::new_empty(syn_num, 0, self.id);
            info!("reply with {:?}", resp);
            self.transport.send(resp)?;
        }
    }
}
fn assemble_cur_buffer(data: &mut [u8], inner: &[u8], base: usize) {
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
