#![allow(dead_code)]

use alloc::{slice, string::String};
use thiserror_no_std::Error;

/// A simple static bytes abstraction inspired by https://github.com/tokio-rs/bytes/
/// In kernel, all written pointers are static
pub(crate) struct BytesMut {
    pub(crate) ptr: *mut u8,
    /// Actual length, <= capacity
    len: usize,
    /// Max capacity, immutable
    capacity: usize,
}

#[derive(Error, Debug)]
pub(crate) enum Error {
    #[error("no space left in the buffer, considering create a new one with larger capacity")]
    NoSpace,
    #[error("allocate failed, {0}")]
    Allocate(String),
}

impl BytesMut {
    #[cfg(feature = "user")]
    pub(crate) fn with_capacity(size: usize) -> Result<Self, Error> {
        use alloc::string::ToString;

        let ptr = unsafe {
            let ptr = libc::malloc(size);
            if ptr.is_null() {
                return Err(Error::Allocate("malloc failed".to_string()));
            }
            ptr
        };
        Ok(Self {
            ptr: ptr as _,
            len: 0,
            capacity: size,
        })
    }

    pub(crate) fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.ptr, self.len) }
    }

    pub(crate) fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }

    pub(crate) fn resize(&mut self, len: usize) -> Result<(), Error> {
        if self.capacity < len {
            Err(Error::NoSpace)
        } else {
            self.len = len;
            Ok(())
        }
    }

    pub(crate) fn write(&mut self, buf: &[u8]) -> Result<(), Error> {
        if buf.len() + self.len > self.capacity {
            Err(Error::NoSpace)
        } else {
            let start = self.len;
            let end = self.len + buf.len();
            self.as_mut_slice()[start..end].copy_from_slice(buf);
            Ok(())
        }
    }
}

impl Drop for BytesMut {
    #[cfg(feature = "user")]
    fn drop(&mut self) {
        unsafe { libc::free(self.ptr as _) }
    }
}

// TODO: in kernel, use kmalloc
