use std::{
    fmt::{Arguments, Write},
    mem::size_of,
};

use bumpalo::Bump;

use crate::{
    serialize::{DecodeEachFn, DecodeFn, Serialize},
    utils::{any_as_bytes, unlikely},
    BumpString,
};

use super::{FlushError, FmtArgHeader, LogArgType, Producer, QueueError, SerializeArgHeader};

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
    #[inline]
    fn bytes_required() -> usize
    where
        Self: Sized,
    {
        size_of::<Self>()
    }
}

impl ChunkRead for usize {
    fn read(buf: &[u8]) -> ReadResult<Self> {
        let (chunk, _) = buf.split_at(<Self as ChunkRead>::bytes_required());
        Ok(usize::from_le_bytes(chunk.try_into().unwrap()))
    }
}

impl ChunkRead for DecodeFn {
    fn read(buf: &[u8]) -> ReadResult<Self> {
        let (chunk, _) = buf.split_at(<Self as ChunkRead>::bytes_required());
        Ok(unsafe { std::mem::transmute(usize::from_le_bytes(chunk.try_into().unwrap())) })
    }
}

impl ChunkRead for DecodeEachFn {
    fn read(buf: &[u8]) -> ReadResult<Self> {
        let (chunk, _) = buf.split_at(<Self as ChunkRead>::bytes_required());
        Ok(unsafe { std::mem::transmute(usize::from_le_bytes(chunk.try_into().unwrap())) })
    }
}

/// Helper trait to allow writing arbitrary types into a byte slice.
pub trait ChunkWrite {
    /// Writes an implementing type into the buffer.
    ///
    /// NOTE: this assumes that `buf` has sufficient capacity.
    #[inline]
    fn write(&self, buf: &mut [u8]) -> usize
    where
        Self: Sized,
    {
        let to_copy = any_as_bytes(self);
        let copy_len = to_copy.len();
        debug_assert!(buf.len() >= copy_len);

        // SAFETY: We requested the exact amount required from the queue, so
        // should not run out of space here.
        unsafe {
            buf.as_mut_ptr()
                .copy_from_nonoverlapping(to_copy.as_ptr(), copy_len);
        }

        copy_len
    }
}

impl<T: Serialize> ChunkWrite for T {
    #[inline]
    fn write(&self, buf: &mut [u8]) -> usize {
        let buf_len = buf.len();
        let rest = self.encode(buf);

        buf_len - rest.len()
    }
}

impl ChunkWrite for &[u8] {
    #[inline]
    fn write(&self, buf: &mut [u8]) -> usize {
        let n = self.len();
        debug_assert!(buf.len() >= n);

        // SAFETY: We requested the exact amount required from the queue, so
        // should not run out of space here.
        unsafe {
            buf.as_mut_ptr().copy_from_nonoverlapping(self.as_ptr(), n);
        }

        n
    }
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
    #[inline]
    pub(crate) fn write_serialize<T: Serialize>(&mut self, arg: &T) {
        let header = SerializeArgHeader {
            type_of_arg: LogArgType::Serialize,
            size_of_arg: arg.buffer_size_required(),
            decode_fn: <T as Serialize>::decode as usize,
        };
        self.write(&header);
        self.write(arg);
    }

    /// Writes a formatted string along with its header.
    #[inline]
    pub(crate) fn write_str(&mut self, s: impl AsRef<str>) {
        let formatted = s.as_ref();
        let header = FmtArgHeader {
            type_of_arg: LogArgType::Fmt,
            size_of_arg: formatted.len(),
        };
        self.write(&header);
        self.write(&formatted.as_bytes());
    }

    /// Writes a type implementing [`ChunkWrite`] by writing through the
    /// underlying buffer(s).
    #[inline]
    pub(crate) fn write<T: ChunkWrite>(&mut self, arg: &T) {
        let buf = self.remaining_slice();
        let written = arg.write(buf);
        self.pos += written;
    }

    #[inline]
    fn remaining_slice(&mut self) -> &mut [u8] {
        let len = self.pos.min(self.inner.len());
        &mut self.inner[len..]
    }
}

/// Contains data needed in preparation for writing to the queue.
pub struct WriteState<T> {
    pub(crate) state: T,
}

/// Preparation stage of writing to the queue.
pub struct WritePrepare<'write> {
    pub(crate) producer: &'write mut Producer,
    pub(crate) fmt_buffer: &'write Bump,
    pub(crate) formatted: bool,
}

impl<'write> WriteState<WritePrepare<'write>> {
    /// Consumes self to signify start of write to queue -- all arguments should
    /// have been preprocessed (if required) and required sizes computed by this
    /// point.
    #[inline]
    pub fn start_write(self, n: usize) -> Result<WriteState<WriteInProgress<'write>>, QueueError> {
        let buf = self.state.producer.prepare_write(n)?;

        Ok(WriteState {
            state: WriteInProgress {
                buffer: Cursor::new(buf),
                formatted: self.state.formatted,
            },
        })
    }

    /// Allocates a formatted [`bumpalo`] string.
    #[inline]
    pub fn format_in(&mut self, args: Arguments) -> BumpString<'write> {
        self.state.formatted = true;

        let mut s = BumpString::with_capacity_in(2048, self.state.fmt_buffer);
        s.write_fmt(args).unwrap();
        s
    }
}

/// In the midst of writing to the queue.
pub struct WriteInProgress<'write> {
    buffer: Cursor<&'write mut [u8]>,
    formatted: bool,
}

impl<'write> WriteState<WriteInProgress<'write>> {
    #[inline]
    pub fn write_serialize<T: Serialize>(&mut self, arg: &T) {
        self.state.buffer.write_serialize(arg);
    }

    #[inline]
    pub fn write_str(&mut self, s: impl AsRef<str>) {
        self.state.buffer.write_str(s);
    }

    #[inline]
    pub fn write<T: ChunkWrite>(&mut self, arg: &T) {
        self.state.buffer.write(arg);
    }

    #[inline]
    pub fn finish(self) -> WriteState<WriteFinish> {
        WriteState {
            state: WriteFinish {
                written: self.state.buffer.finish(),
                formatted: self.state.formatted,
            },
        }
    }
}

/// Finished writing to the queue.
pub struct WriteFinish {
    pub(crate) written: usize,
    pub(crate) formatted: bool,
}
