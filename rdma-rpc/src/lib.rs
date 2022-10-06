use KRdmaKit::*;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rdma_works() {
        println!("Num RDMA devices found: {}", UDriver::create().unwrap().devices().len()); 
    }
}
