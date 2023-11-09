use rtrb::{chunks::ReadChunk, Consumer};

use super::{ReadError, ReadResult};

/// Consumer side of queue.
pub(crate) struct Receiver(pub(crate) Consumer<u8>);

impl Receiver {
    pub(crate) fn read_chunk(&mut self) -> ReadResult<ReadChunk<'_, u8>> {
        self.0
            .read_chunk(self.0.slots())
            .map_err(|_| ReadError::NotEnoughBytes)
    }
}
