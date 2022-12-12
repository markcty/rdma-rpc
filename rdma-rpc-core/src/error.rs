use alloc::string::String;
use thiserror_no_std::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to connect to the server")]
    Connect,
    #[error("failed to encode rpc args")]
    EncodeArgs,
    #[error("failed to decode rpc args")]
    DecodeArgs,
    #[error("failed to decode rpc response")]
    DecodeResp,
    #[error("internal error, {0}")]
    Internal(String),
}
