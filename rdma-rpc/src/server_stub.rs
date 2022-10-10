use alloc::string::String;

/// Server stub for RPC call
pub trait ServerStub {
    /// Register rpc handler, bound with a rpc_id
    fn register_rpc_handler(rpc_id: u64) -> Result<(), ()>;

    /// Start an event loop, wait for requests
    fn run_event_loop() -> Result<(), ()>;
}
