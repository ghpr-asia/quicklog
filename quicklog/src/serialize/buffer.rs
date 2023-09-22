use crate::constants::MAX_SERIALIZE_BUFFER_CAPACITY;

/// Bytebuffer to provide byte chunks for store
pub struct ByteBuffer {
    data: Vec<u8>,
    write_idx: usize,
}

impl ByteBuffer {
    pub fn new() -> Self {
        let mut data = Vec::new();
        data.resize(MAX_SERIALIZE_BUFFER_CAPACITY, 0);
        Self { data, write_idx: 0 }
    }

    pub fn get_chunk_as_mut(&mut self, chunk_size: usize) -> &mut [u8] {
        let curr_idx = self.write_idx;
        if chunk_size > MAX_SERIALIZE_BUFFER_CAPACITY {
            panic!(
                "BUFFER size insufficient to support chunk_size: {}, please increase MAX_CAPACITY",
                chunk_size
            );
        }

        // This condition guards against the case where the amount of data we want to write
        // is greater than the MAX_SERIALIZE_BUFFER_CAPACITY. When this happens,
        // it is possible that the initial log lines before the one that caused this overflow
        // will be wrong. This is EXPECTED.
        // When this happens, the user should modify the BUFFER_SIZE
        if curr_idx + chunk_size > MAX_SERIALIZE_BUFFER_CAPACITY {
            self.write_idx = chunk_size;
            // in release, overwrite existing items without panic
            &mut self.data[0..chunk_size]
        } else {
            self.write_idx += chunk_size;
            &mut self.data[curr_idx..curr_idx + chunk_size]
        }
    }
}

impl Default for ByteBuffer {
    fn default() -> Self {
        Self::new()
    }
}
