pub const ND_CACHE__INCOMPLETE_RETRY_TIME: u64 = 1;
pub const ND_CACHE__INCOMPLETE_RETRY_LIMIT: usize = 2;
pub const ND_CACHE__REACHABLE_TIME: u64 = 45;
pub const ND_CACHE__DELAY_TIME: u64 = 5;
pub const ND_CACHE__PROBE_RETRY_TIME: u64 = 1;
pub const ND_CACHE__PROBE_RETRY_LIMIT: usize = 2;
pub const ND_CACHE__TIME_LOOP_DELAY: u64 = 100;

const WORDS: &str = "hello convenience!";
pub const LOCAL_HOST: &str = "localhost:3333";
pub const IP6_DAD_RETRIES: usize = 3;
pub const IP6_DAD_DELAY: u64 = 500;

pub const FRAME_CONTENT_MAX_LEN: usize = 2048;
pub const MAX_BUFF: usize = 128;
pub const CONNECT_PASSWORD: u32 = 99;
pub const END_ACK: u32 = 40;
pub const MAX_MSG_SIZE: usize = 512;
pub const WINDOW_SIZE: usize = 4;

pub enum SendCase {
    Normal,
    MayLoss,
    MayOverTime,
}
pub const SEND_CASE: SendCase = SendCase::Normal;
// pub const SEND_CASE: SendCase = SendCase::MayLoss;
