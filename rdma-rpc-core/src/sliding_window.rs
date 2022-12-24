use core::cmp::min;

/// Sliding window on a slice
/// # Examples
///
/// ```
/// # use rdma_rpc_core::sliding_window::Window;
/// let data = [1, 2, 3];
/// let mut window = Window::new(data.as_slice(), 2);
/// assert_eq!(&[1, 2], window.get());
/// window.slide();
/// assert_eq!(&[2, 3], window.get());
/// window.slide();
/// assert_eq!(&[3], window.get());
/// window.slide();
/// assert!(window.is_closed());
/// ```
///
/// # Panics
/// Call to methods other than `is_closed` will panic if window has closed(moved to the end of the slice)
pub struct SlidingWindow<'a, T> {
    size: usize,
    slice: &'a [T],
    p: usize,
}

impl<'a, T> SlidingWindow<'a, T> {
    pub fn new(slice: &'a [T], size: usize) -> Self {
        Self { size, slice, p: 0 }
    }

    pub fn get(&self) -> &'a [T] {
        assert!(self.p <= self.slice.len(), "window has closed");

        let base = self.p;
        let upper = min(self.p + self.size, self.slice.len());
        &self.slice[base..upper]
    }

    pub fn first(&self) -> &'a T {
        assert!(self.p <= self.slice.len(), "window has closed");

        &self.slice[self.p]
    }

    pub fn last(&self) -> &'a T {
        assert!(self.p <= self.slice.len(), "window has closed");

        let upper = min(self.p + self.size, self.slice.len());
        &self.slice[upper - 1]
    }

    pub fn slide(&mut self) {
        assert!(self.p < self.slice.len(), "window has closed");

        self.p += 1;
    }

    pub fn is_closed(&self) -> bool {
        self.p == self.slice.len()
    }
}
