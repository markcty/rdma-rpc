pub const RESPONSE_SIZE: usize = 4;
pub const MESSAGE_CONTENT_SIZE: usize = 4;
pub const MESSAGE_SIZE: usize = 16;

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct Response {
    pub seq_num: u32,
    pub ack_num: u32,
    pub check_sum: u32,
}
impl Response {
    pub fn new(seq_num: u32, ack_num: u32) -> Response {
        let mut ret_resp = Response {
            seq_num,
            ack_num,
            check_sum: 0u32,
        };
        ret_resp.set_checksum();
        ret_resp
    }
    pub fn set_checksum(&mut self) {
        self.check_sum = self.gen_checksum()
    }
    pub fn check_checksum(&self) -> bool {
        self.check_sum == self.gen_checksum()
    }
    pub fn gen_checksum(&self) -> u32 {
        let mut check_sum = 0u32;
        check_sum += self.seq_num;
        check_sum += self.ack_num;
        check_sum = (check_sum >> 16) + check_sum & 0xFF_FF;
        !check_sum
    }
}

pub struct TCP_Segment {
    pub src_port: u32,
    pub des_port: u32,
    pub seq_number: u64,
    pub ack_number: u64,
    pub data_offset: u16,
    pub window_size: u32,
    pub check_sum: u32,
}
#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct Message {
    pub seq_num: u32,
    pub ack_num: u32,
    pub check_sum: u32,
    pub content: [u8; MESSAGE_CONTENT_SIZE],
}

impl Message {
    pub fn new(seq_num: u32, ack_num: u32, content: [u8; MESSAGE_CONTENT_SIZE]) -> Message {
        let mut ret_msg = Message {
            seq_num,
            ack_num,
            check_sum: 0u32,
            content,
        };
        ret_msg.set_checksum();
        ret_msg
    }
    pub fn set_checksum(&mut self) {
        self.check_sum = self.gen_checksum()
    }
    pub fn check_checksum(&self) -> bool {
        self.check_sum == self.gen_checksum()
    }
    pub fn gen_checksum(&self) -> u32 {
        let mut check_sum = 0u32;
        let buf_len = self.content.len();
        let mut idx = 0;
        check_sum += self.seq_num;
        check_sum += self.ack_num;
        while idx + 1 < buf_len {
            check_sum += (self.content[idx] as u32) << 8;
            check_sum += self.content[idx + 1] as u32;
            idx += 2;
        }
        if idx < buf_len {
            check_sum += (self.content[idx] as u32) << 8;
        }
        check_sum = (check_sum >> 16) + check_sum & 0xFF_FF;
        !check_sum
    }
}
pub unsafe fn serialize_any<T: Sized>(src: &T) -> &[u8] {
    ::std::slice::from_raw_parts((src as *const T) as *const u8, ::std::mem::size_of::<T>())
}
pub unsafe fn deserialize_message(src: &[u8]) -> Message {
    std::ptr::read(src.as_ptr() as *const _)
}
pub unsafe fn deserialize_response(src: &[u8]) -> Response {
    std::ptr::read(src.as_ptr() as *const _)
}
