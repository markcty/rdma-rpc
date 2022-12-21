#![no_std]
#![feature(slice_as_chunks)]
#![feature(let_chains)]

extern crate alloc;

pub mod client_stub;
pub mod error;
pub(crate) mod message_buffer;
pub mod messages;
pub mod server_stub;
pub mod session;
pub mod transport;

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
