use alloc::string::{String, ToString};

use thiserror_no_std::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to connect to the server")]
    Connect,
    #[error("failed to encode rpc args")]
    DecodeEncode(String),
    #[error("failed to decode rpc response")]
    DecodeResp,
    #[error("internal error, {0}")]
    Internal(String),
    #[error("receive error")]
    Receive,
    #[error("timeout")]
    Timeout,
}

impl From<bincode::Error> for Error {
    fn from(err: bincode::Error) -> Self {
        Self::DecodeEncode(err.to_string())
    }
}
