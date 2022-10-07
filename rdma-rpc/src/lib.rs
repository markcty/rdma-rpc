#![no_std]

use KRdmaKit::*;

pub mod transport;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rdma_works() {
        KRdmaKit::log::info!("Num RDMA devices found: {}", UDriver::create().unwrap().devices().len()); 
    }
}
