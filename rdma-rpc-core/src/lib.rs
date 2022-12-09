#![no_std]

extern crate alloc;

pub mod client_stub;
pub mod error;
pub(crate) mod message_buffer;
pub mod messages;
pub mod server_stub;
pub mod session;
pub mod transport;
pub(crate) mod utils;

pub type RpcId = u64;

#[cfg(test)]
mod tests {
    use KRdmaKit::UDriver;

    #[test]
    fn rdma_works() {
        KRdmaKit::log::info!(
            "Num RDMA devices found: {}",
            UDriver::create().unwrap().devices().len()
        );
    }
}
