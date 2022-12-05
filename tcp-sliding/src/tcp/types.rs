pub struct Response {
    pub ack: u32,
}
pub struct Message {
    pub id: u32,
    pub context: [u8; 255],
}
#[repr(C)]
pub struct Row {
    id: u32,
    username: [u8; 32],
    email: [u8; 255],
}
pub unsafe fn serialize_row<T: Sized>(src: &T) -> &[u8] {
    ::std::slice::from_raw_parts((src as *const T) as *const u8, ::std::mem::size_of::<T>())
}
pub unsafe fn deserialize_row(src: Vec<u8>) -> Row {
    std::ptr::read(src.as_ptr() as *const _)
}
