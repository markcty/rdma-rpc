use alloc::string::String;

/// Server stub for RPC call
pub trait ServerStub {
    fn register_rpc_handler(rpc_id: u64) -> Result<(), ()>;
    fn run_event_loop() -> Result<(), ()>;
}
