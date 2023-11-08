use rtrb::{chunks::WriteChunkUninit, Producer};

use super::WriteError;

/// Producer side of queue
pub(crate) struct Sender(pub(crate) Producer<u8>);

impl Sender {
    pub(crate) fn write_chunk(&mut self) -> Result<WriteChunkUninit<'_, u8>, WriteError> {
        self.0
            .write_chunk_uninit(self.0.slots())
            .map_err(|_| WriteError::NotEnoughSpace)
    }
}
