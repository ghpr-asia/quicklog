use crate::constants::MAX_SERIALIZE_BUFFER_CAPACITY_BYTES;
use once_cell::unsync::OnceCell;

static mut BYTE_BUFFER: [u8; MAX_SERIALIZE_BUFFER_CAPACITY_BYTES] =
    [0_u8; MAX_SERIALIZE_BUFFER_CAPACITY_BYTES];

pub static mut BUFFER: OnceCell<Buffer> = OnceCell::new();

cfg_if::cfg_if! {
    if #[cfg(debug_assertions)] {
        use std::sync::atomic::{AtomicUsize, Ordering};

        /// In debug, Buffer has an atomic read and write index which tracks if the write index
        /// overruns the read index, or if the logging happens faster than the flushing.
        ///
        /// It panics when logging happens faster than flushing, which is an indication we need
        /// a larger buffer size.
        pub struct Buffer {
            write_idx: AtomicUsize,
            read_idx: AtomicUsize,
        }

        impl Buffer {
            pub fn new() -> Buffer {
                Buffer {
                    /// monotonically increasing idx
                    write_idx: AtomicUsize::new(0),
                    read_idx: AtomicUsize::new(0),
                }
            }

            /// updates the read index and checks if read has overrun write
            pub fn dealloc(&mut self, chunk_size: usize) {
                let curr_read = self.read_idx.fetch_add(chunk_size, Ordering::Release) + chunk_size;
                let curr_write = self.write_idx.load(Ordering::Acquire);

                assert!(
                    curr_read <= curr_write,
                    // TODO: State which env var to change to amend the buffer capacity
                    "read index is greater than write index, this means logging is happening faster than flushing, you might want to increase the buffer capacity"
                );
            }

            pub fn get_chunk_as_mut(&mut self, chunk_size: usize) -> &'static mut [u8] {
                let curr_write = self.write_idx.load(Ordering::Acquire);
                let curr_write_wrapped = curr_write % MAX_SERIALIZE_BUFFER_CAPACITY_BYTES;
                let write_quotient = curr_write / MAX_SERIALIZE_BUFFER_CAPACITY_BYTES;

                if chunk_size > MAX_SERIALIZE_BUFFER_CAPACITY_BYTES {
                    panic!(
                        "BUFFER size insufficient to support chunk_size: {}, please increase max serialize buffer capacity",
                        chunk_size
                    );
                }

                let curr_read = self.read_idx.load(Ordering::Acquire);
                let curr_read_wrapped = curr_read % MAX_SERIALIZE_BUFFER_CAPACITY_BYTES;
                let read_quotient = curr_read / MAX_SERIALIZE_BUFFER_CAPACITY_BYTES;

                // assert actual end will never be greater than start
                assert!(curr_read <= curr_write, "read idx should never be greater than write idx");

                // case 1: equal number of wrap-arounds, means write must be ahead or equal
                if read_quotient == write_quotient {
                    assert!(curr_write_wrapped >= curr_read_wrapped, "write idx will overrun read idx, please increase serialize buffer capacity");
                    // case 1a: we don't need to wrap around after this new chunk has been added
                    if curr_write_wrapped + chunk_size <= MAX_SERIALIZE_BUFFER_CAPACITY_BYTES {
                        self.write_idx.fetch_add(chunk_size, Ordering::Release);
                        unsafe { &mut BYTE_BUFFER[curr_write_wrapped..curr_write_wrapped + chunk_size]}
                    } else {
                        // case 1b: need to wrap around, check that will not overrun
                        assert!(chunk_size < curr_read_wrapped, "write idx will overrun read idx, please increase serialize buffer capacity");
                        self.write_idx.fetch_add(chunk_size + MAX_SERIALIZE_BUFFER_CAPACITY_BYTES - curr_write_wrapped, Ordering::Release);
                        unsafe { &mut BYTE_BUFFER[0..chunk_size]}
                    }
                } else {
                    // case 2: write wrapped around, but read has not
                    // ensure that write idx is lesser than read idx
                    assert!(curr_write_wrapped + chunk_size < curr_read_wrapped, "write idx will overrun read idx, please increase serialize buffer capacity");
                    self.write_idx.fetch_add(chunk_size, Ordering::Release);
                    unsafe { &mut BYTE_BUFFER[curr_write_wrapped..curr_write_wrapped + chunk_size]}
                }
            }
        }
    } else {
        /// In release, buffer only has a write_idx and there's no overhead
        /// of using atomics
        pub struct Buffer {
            write_idx: usize,
        }

        impl Buffer {
            pub fn new() -> Buffer {
                Buffer {
                    write_idx: 0
                }
            }

            pub fn get_chunk_as_mut(&mut self, chunk_size: usize) -> &'static mut [u8] {
                let curr_idx = self.write_idx;

                if chunk_size > MAX_SERIALIZE_BUFFER_CAPACITY_BYTES {
                    panic!(
                        "BUFFER size insufficient to support chunk_size: {}, please increase MAX_CAPACITY",
                        chunk_size
                    );
                }

                // loop back around if insufficient size
                if curr_idx + chunk_size > MAX_SERIALIZE_BUFFER_CAPACITY_BYTES {
                    self.write_idx = chunk_size;
                    // in release, overwrite existing items without panic
                    unsafe { &mut BYTE_BUFFER[0..chunk_size] }
                } else {
                    self.write_idx += chunk_size;
                    unsafe { &mut BYTE_BUFFER[curr_idx..curr_idx + chunk_size] }
                }
            }
        }
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}
