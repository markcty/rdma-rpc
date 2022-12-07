pub const RESPONSE_SIZE: usize = 4;
pub const MESSAGE_CONTENT_SIZE: usize = 4;
pub const MESSAGE_SIZE: usize = 8;

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct Response {
    pub ack: u32,
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct Message {
    pub id: u32,
    pub content: [u8; MESSAGE_CONTENT_SIZE],
}
pub unsafe fn serialize_any<T: Sized>(src: &T) -> &[u8] {
    ::std::slice::from_raw_parts((src as *const T) as *const u8, ::std::mem::size_of::<T>())
}
pub unsafe fn deserialize_message(src: &[u8]) -> Message {
    std::ptr::read(src.as_ptr() as *const _)
}
pub unsafe fn deserialize_response(src: &[u8]) -> Response {
    std::ptr::read(src.as_ptr() as *const _)
}
