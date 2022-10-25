#![no_std]

extern crate alloc;

use KRdmaKit::*;

pub mod transport;
pub mod server_stub;
pub mod client_stub;
pub mod message_buffer;
pub mod session;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rdma_works() {
        KRdmaKit::log::info!("Num RDMA devices found: {}", UDriver::create().unwrap().devices().len()); 
    }
}
