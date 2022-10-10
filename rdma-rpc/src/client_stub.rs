use crate::message_buffer::{MessageBuffer, BytesMut};

/// Client stub for RPC call
pub trait ClientStub {
    /*
        Create session with the remote server in `create`
     */
    fn create(addr: String) -> Result<Self, ()>;
    /*
        `sync_call` call a rpc_id synchronously
     */
    fn sync_call(rpc_id: u64, input: &BytesMut) -> Result<BytesMut, ()>;
}
