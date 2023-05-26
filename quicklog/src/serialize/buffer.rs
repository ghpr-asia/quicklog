use crate::constants::MAX_SERIALIZE_BUFFER_CAPACITY;
use once_cell::unsync::OnceCell;

static mut BYTE_BUFFER: [u8; MAX_SERIALIZE_BUFFER_CAPACITY] = [0_u8; MAX_SERIALIZE_BUFFER_CAPACITY];

pub static mut BUFFER: OnceCell<Buffer> = OnceCell::new();

/// In release, buffer only has a write_idx and there's no overhead
/// of using atomics
pub struct Buffer {
    write_idx: usize,
}

impl Buffer {
    pub fn new() -> Buffer {
        Buffer { write_idx: 0 }
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

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}
