use crate::message_buffer::BytesMut;

/// Session provides send/receive between server/client
pub trait Session {
    /* 
        Session should act like a stream. Users will read/write from this object.
        Should handle reorder and package loss.
     */
    const MTU: usize;

    fn id(&self) -> u64;
    fn send(&self, buffer: &mut BytesMut);
    fn receive(&self, buffer: &BytesMut);
}

/// Session manager manages all the sessions in a server
pub struct SessionManager {

}
