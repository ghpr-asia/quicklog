use std::mem::size_of;

use crate::{
    serialize::{DecodeEachFn, DecodeFn, Serialize},
    utils::unlikely,
};

use super::{FlushError, FmtArgHeader, LogArgType, SerializeArgHeader};

/// Result from pushing onto queue.
pub type WriteResult<T> = Result<T, WriteError>;

/// Result from reading from queue.
pub type ReadResult<T> = Result<T, ReadError>;

/// Helper trait to allow for reading arbitrary types from a byte slice.
pub trait ChunkRead
where
    Self: Sized,
{
    /// Given a byte slice, parses sufficient bytes to reconstruct the
    /// implementing type.
    ///
    /// NOTE: this assumes that `buf` has sufficient capacity.
    fn read(buf: &[u8]) -> ReadResult<Self>;
    /// The amount of bytes required to reconstruct the implementing type.
    fn bytes_required() -> usize;
}

impl ChunkRead for usize {
    fn read(buf: &[u8]) -> ReadResult<Self> {
        let (chunk, _) = buf.split_at(<Self as ChunkRead>::bytes_required());
        Ok(usize::from_le_bytes(chunk.try_into().unwrap()))
    }

    fn bytes_required() -> usize {
        size_of::<usize>()
    }
}

impl ChunkRead for DecodeFn {
    fn read(buf: &[u8]) -> ReadResult<Self> {
        let (chunk, _) = buf.split_at(<Self as ChunkRead>::bytes_required());
        Ok(unsafe { std::mem::transmute(usize::from_le_bytes(chunk.try_into().unwrap())) })
    }

    fn bytes_required() -> usize {
        size_of::<Self>()
    }
}

impl ChunkRead for DecodeEachFn {
    fn read(buf: &[u8]) -> ReadResult<Self> {
        let (chunk, _) = buf.split_at(<Self as ChunkRead>::bytes_required());
        Ok(unsafe { std::mem::transmute(usize::from_le_bytes(chunk.try_into().unwrap())) })
    }

    fn bytes_required() -> usize {
        size_of::<Self>()
    }
}

/// Helper trait to allow writing arbitrary types into a byte slice.
pub trait ChunkWrite {
    /// Writes an implementing type into the buffer.
    ///
    /// NOTE: this assumes that `buf` has sufficient capacity.
    fn write(&self, buf: &mut [u8]) -> usize;
    /// The amount of bytes required to write the implementing type.
    fn bytes_required(&self) -> usize;
}

impl<T: Serialize> ChunkWrite for T {
    fn write(&self, buf: &mut [u8]) -> usize {
        let (store, _) = self.encode(buf);
        store.buffer.len()
    }

    fn bytes_required(&self) -> usize {
        self.buffer_size_required()
    }
}

impl ChunkWrite for &[u8] {
    fn write(&self, buf: &mut [u8]) -> usize {
        let n = self.len();
        let (chunk, _) = buf.split_at_mut(n);
        chunk.copy_from_slice(self);

        n
    }

    fn bytes_required(&self) -> usize {
        self.len()
    }
}

/// Error writing to the queue.
#[derive(Copy, Clone, Debug)]
pub enum WriteError {
    /// Queue does not have sufficient capacity to be written to.
    NotEnoughSpace,
}

/// Error reading from the queue.
#[derive(Copy, Clone, Debug)]
pub enum ReadError {
    /// Queue does not have sufficient valid bytes left to read the required
    /// value.
    NotEnoughBytes,
    /// Value parsed from the queue does not match expected value.
    UnexpectedValue,
}

impl From<ReadError> for FlushError {
    fn from(value: ReadError) -> Self {
        match value {
            ReadError::NotEnoughBytes => Self::InsufficientSpace,
            ReadError::UnexpectedValue => Self::Decode,
        }
    }
}

/// Error advancing [`Cursor`] through the queue.
pub enum CursorError {
    /// Queue does not have any space left to advance through.
    NoSpaceLeft,
}

impl From<CursorError> for ReadError {
    fn from(value: CursorError) -> Self {
        match value {
            CursorError::NoSpaceLeft => Self::NotEnoughBytes,
        }
    }
}

/// Similar to [`std::io::Cursor`], but we implement our own methods to aid in
/// writing structured data to the buffer.
pub struct Cursor<T> {
    inner: T,
    pos: usize,
}

impl<T> Cursor<T> {
    pub fn new(inner: T) -> Self {
        Self { inner, pos: 0 }
    }

    pub fn finish(self) -> usize {
        self.pos
    }
}

impl<T> Cursor<T>
where
    T: AsRef<[u8]>,
{
    /// Whether there are remaining bytes to read.
    pub fn is_empty(&self) -> bool {
        self.remaining_size() == 0
    }

    /// Remaining bytes to read.
    pub fn remaining_size(&self) -> usize {
        self.inner.as_ref().len() - self.pos
    }

    /// Reconstructs a type implementing [`ChunkRead`] by reading through the
    /// underlying buffer(s).
    pub fn read<C: ChunkRead>(&mut self) -> ReadResult<C> {
        let required_size = C::bytes_required();
        let inner = self.inner.as_ref();
        let buf = {
            let len = self.pos.min(inner.len());
            &inner[len..]
        };
        if unlikely(buf.len() < required_size) {
            return Err(ReadError::NotEnoughBytes);
        }

        let res = C::read(buf)?;
        self.pos += required_size;

        Ok(res)
    }

    /// Reads `n` bytes from the underlying buffer(s), and returns a slice if
    /// there is enough capacity.
    pub fn read_bytes(&mut self, n: usize) -> ReadResult<&[u8]> {
        let inner = self.inner.as_ref();
        let buf = {
            let len = self.pos.min(inner.len());
            &inner[len..]
        };
        if unlikely(buf.len() < n) {
            return Err(ReadError::NotEnoughBytes);
        }
        let res = unsafe { buf.get_unchecked(..n) };
        self.pos += n;

        Ok(res)
    }

    pub fn read_decode_each(
        &mut self,
        decode_fn: DecodeEachFn,
        out: &mut Vec<String>,
    ) -> ReadResult<()> {
        let inner = self.inner.as_ref();
        let buf = {
            let len = self.pos.min(inner.len());
            &inner[len..]
        };
        let rest = decode_fn(buf, out);
        self.pos += buf.len() - rest.len();

        Ok(())
    }
}

impl Cursor<&mut [u8]> {
    /// Writes an argument implementing [`Serialize`], along with its header.
    pub fn write_serialize<T: Serialize>(&mut self, arg: &T) -> WriteResult<()> {
        let header = SerializeArgHeader {
            type_of_arg: LogArgType::Serialize,
            size_of_arg: arg.buffer_size_required(),
            decode_fn: <T as Serialize>::decode as usize,
        };
        self.write(&header)?;
        self.write(arg)
    }

    /// Writes a formatted string along with its header.
    pub fn write_str(&mut self, s: impl AsRef<str>) -> WriteResult<()> {
        let formatted = s.as_ref();
        let header = FmtArgHeader {
            type_of_arg: LogArgType::Fmt,
            size_of_arg: formatted.len(),
        };
        self.write(&header)?;
        self.write(&formatted.as_bytes())?;

        Ok(())
    }

    /// Writes a type implementing [`ChunkWrite`] by writing through the
    /// underlying buffer(s).
    pub fn write<T: ChunkWrite>(&mut self, arg: &T) -> WriteResult<()> {
        let required_size = arg.bytes_required();
        let buf = self.remaining_slice();
        if unlikely(buf.len() < required_size) {
            return Err(WriteError::NotEnoughSpace);
        }

        let written = arg.write(buf);
        self.pos += written;
        Ok(())
    }

    #[inline]
    pub fn remaining_slice(&mut self) -> &mut [u8] {
        let len = self.pos.min(self.inner.len());
        &mut self.inner[len..]
    }
}
