/// A simple static bytes abstraction inspired by https://github.com/tokio-rs/bytes/
/// In kernel, all written pointers are static
pub struct BytesMut {
    pub(crate) ptr: *mut u8,
    len: usize,
}

/// Allocate `BytesMut` with different allocator (e.g.: kmalloc, vmalloc in kernel)
pub trait Allocator {

}
