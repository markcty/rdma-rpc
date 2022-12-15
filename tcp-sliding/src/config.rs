use std::time::Duration;

pub const LOCAL_HOST: &str = "localhost:3333";

pub const FRAME_CONTENT_MAX_LEN: usize = 2048;
pub const MAX_BUFF: usize = 128;
pub const SIZE_ACK: u32 = 101;
pub const CONNECT_PASSWORD: u32 = 99;
pub const END_ACK: u32 = 40;
pub const MAX_MSG_SIZE: usize = 512;
pub const WINDOW_SIZE: usize = 4;
pub const TIME_OUT: Duration = Duration::from_secs(5);

pub enum SendCase {
    Normal,
    MayLoss,
    MayOverTime,
    MustLoss,
}
pub const LOSS_RATE: usize = 20;
pub const SEND_CASE: SendCase = SendCase::Normal;
// pub const SEND_CASE: SendCase = SendCase::MustLoss;
// pub const SEND_CASE: SendCase = SendCase::MayLoss;
