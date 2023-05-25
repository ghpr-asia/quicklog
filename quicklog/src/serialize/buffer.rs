use crate::constants::MAX_SERIALIZE_BUFFER_CAPACITY;
use once_cell::unsync::OnceCell;

static mut BYTE_BUFFER: [u8; MAX_SERIALIZE_BUFFER_CAPACITY] = [0_u8; MAX_SERIALIZE_BUFFER_CAPACITY];

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
                    write_idx: AtomicUsize::new(0),
                    read_idx: AtomicUsize::new(0),
                }
            }

            /// updates the read index and checks if read has overrun write
            pub fn dealloc(&mut self, chunk_size: usize) {
                let new_val = self.read_idx.fetch_add(chunk_size, Ordering::Release);
                if new_val + chunk_size > MAX_SERIALIZE_BUFFER_CAPACITY {
                    // due to behaviour that if we overrun end, we allocate from start,
                    // we update read idx to start instead of end
                    self.read_idx.store(chunk_size, Ordering::Release);
                }

                // in debug mode, assert that our invariant is true, that read will always
                // be less than or equal to write, otherwise we broke the loop
                let curr_write = self.write_idx.load(Ordering::Acquire);
                let curr_read = self.read_idx.load(Ordering::Acquire);
                assert!(
                    curr_read <= curr_write,
                    // TODO: State which env var to change to amend the buffer capacity
                    "read index is greater than write index, this means logging is happening faster than flushing, you might want to increase the buffer capacity"
                );
            }

            pub fn get_chunk_as_mut(&mut self, chunk_size: usize) -> &'static mut [u8] {
                let curr_idx = self.write_idx.load(Ordering::Acquire);

                if chunk_size > MAX_SERIALIZE_BUFFER_CAPACITY {
                    panic!(
                        "BUFFER size insufficient to support chunk_size: {}, please increase MAX_CAPACITY",
                        chunk_size
                    );
                }

                // loop back around if insufficient size
                if curr_idx + chunk_size > MAX_SERIALIZE_BUFFER_CAPACITY {
                    // gives two branches, one for debug, which panics when we overwrite the end with
                    // serialize, and the other for production, which simply overwrites regardless
                    // if in debug, check where we have read up to
                    let curr_end = self.read_idx.load(Ordering::Acquire);
                    if curr_end > chunk_size {
                        self.write_idx.store(chunk_size, Ordering::Release);
                        // safe, we have up to write_idx to alloc and we require less
                        unsafe { &mut BYTE_BUFFER[0..chunk_size] }
                    } else {
                        // unsafe, we will overwrite existing items, panic!
                        panic!("Writing index will overwrite read index! You might need a larger buffer capacity.")
                    }
                } else {
                    // sufficient size before end
                    self.write_idx
                        .store(curr_idx + chunk_size, Ordering::Release);
                    unsafe { &mut BYTE_BUFFER[curr_idx..curr_idx + chunk_size] }
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

                if chunk_size > MAX_SERIALIZE_BUFFER_CAPACITY {
                    panic!(
                        "BUFFER size insufficient to support chunk_size: {}, please increase MAX_CAPACITY",
                        chunk_size
                    );
                }

                // loop back around if insufficient size
                if curr_idx + chunk_size > MAX_SERIALIZE_BUFFER_CAPACITY {
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
