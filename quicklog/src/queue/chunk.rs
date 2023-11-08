use std::{
    fmt::{Arguments, Write},
    mem::{size_of, MaybeUninit},
};

use crate::{
    serialize::{DecodeFn, Serialize},
    utils::likely,
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
    NotEnoughSpace,
}

/// Error reading from the queue.
#[derive(Copy, Clone, Debug)]
pub enum ReadError {
    NotEnoughBytes,
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

/// Error advancing [`CursorRef`] or [`CursorMut`] through the queue.
pub enum CursorError {
    NoSpaceLeft,
}

impl From<CursorError> for ReadError {
    fn from(value: CursorError) -> Self {
        match value {
            CursorError::NoSpaceLeft => Self::NotEnoughBytes,
        }
    }
}

/// Similar to [`std::io::Cursor`], but manages two buffers.
///
/// For the main read operations, if the head slice does not have enough
/// remaining capacity to reconstruct the type, then it moves on to the
/// next slice and continues parsing. Once this happens, subsequent reads
/// made via this cursor instance will read from this second slice.
pub struct CursorRef<'buf> {
    head: &'buf [u8],
    tail: Option<&'buf [u8]>,
}

impl<'buf> CursorRef<'buf> {
    pub fn new(head: &'buf [u8], tail: &'buf [u8]) -> Self {
        CursorRef {
            head,
            tail: Some(tail),
        }
    }

    /// Reconstructs a type implementing [`ChunkRead`] by reading through
    /// the underlying buffer(s).
    pub fn read<T: ChunkRead>(&mut self) -> ReadResult<T> {
        let required_size = T::bytes_required();
        if likely(self.head.len() >= required_size) {
            let result = T::read(self.head)?;
            self.head = &self.head[required_size..];

            Ok(result)
        } else {
            match self.advance(required_size) {
                Ok(()) => {
                    let result = T::read(self.head)?;
                    self.head = &self.head[required_size..];

                    Ok(result)
                }
                Err(e) => Err(e.into()),
            }
        }
    }

    /// Reads `n` bytes from the underlying buffer(s), and returns a slice
    /// if there is enough capacity.
    pub fn read_bytes(&mut self, n: usize) -> ReadResult<&[u8]> {
        if likely(self.head.len() >= n) {
            let (chunk, rest) = self.head.split_at(n);
            self.head = rest;
            Ok(chunk)
        } else {
            match self.advance(n) {
                Ok(()) => {
                    let (chunk, rest) = self.head.split_at(n);
                    self.head = rest;

                    Ok(chunk)
                }
                Err(e) => Err(e.into()),
            }
        }
    }

    /// Whether there are remaining bytes to read.
    pub fn is_empty(&self) -> bool {
        self.remaining_size() == 0
    }

    /// Remaining bytes to read.
    pub fn remaining_size(&self) -> usize {
        self.head.len() + self.tail.as_ref().map(|t| t.len()).unwrap_or(0)
    }

    /// Moves to read from the second underlying buffer, if there is at least
    /// `n` capacity. Otherwise, it means the top-level requested read is not
    /// successful, since we have reached the end of both buffers.
    #[inline]
    fn advance(&mut self, n: usize) -> Result<(), CursorError> {
        match &self.tail {
            Some(t) if t.len() >= n => {
                self.head = self.tail.take().unwrap();
                Ok(())
            }
            _ => Err(CursorError::NoSpaceLeft),
        }
    }
}

/// Same as [`CursorRef`], but for write operations.
pub struct CursorMut<'buf> {
    head: &'buf mut [u8],
    tail: Option<&'buf mut [u8]>,
    written: usize,
}

impl<'buf> CursorMut<'buf> {
    pub fn new(head: &'buf mut [MaybeUninit<u8>], tail: &'buf mut [MaybeUninit<u8>]) -> Self {
        // Eventually we will be overwriting the slices and committing the
        // written bytes, so just assume initialized here
        let (head, tail) = unsafe {
            (
                std::slice::from_raw_parts_mut(head.as_mut_ptr().cast(), head.len()),
                std::slice::from_raw_parts_mut(tail.as_mut_ptr().cast(), tail.len()),
            )
        };
        CursorMut {
            head,
            tail: Some(tail),
            written: 0,
        }
    }

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

    /// Writes a validated format [`Arguments`], along with its header.
    pub fn write_fmt(&mut self, fmt_buffer: &mut String, arg: Arguments<'_>) -> WriteResult<()> {
        // Unwrap: String automatically resizes if needed, so shouldn't
        // fail while formatting
        write!(fmt_buffer, "{}", arg).unwrap();
        let header = FmtArgHeader {
            type_of_arg: LogArgType::Fmt,
            size_of_arg: fmt_buffer.len(),
        };
        self.write(&header)?;
        self.write(&fmt_buffer.as_bytes())?;
        fmt_buffer.clear();

        Ok(())
    }

    /// Writes a type implementing [`ChunkWrite`] by writing through the
    /// underlying buffer(s).
    pub fn write<T: ChunkWrite>(&mut self, arg: &T) -> WriteResult<()> {
        let required_size = arg.bytes_required();
        if likely(self.head.len() >= required_size) {
            let written = arg.write(self.head);
            let head = std::mem::take(&mut self.head);
            self.head = &mut head[written..];
            self.written += written;

            Ok(())
        } else {
            match self.advance(required_size) {
                Ok(()) => {
                    let written = arg.write(self.head);
                    let head = std::mem::take(&mut self.head);
                    self.head = &mut head[written..];
                    self.written += written;

                    Ok(())
                }
                Err(_) => Err(WriteError::NotEnoughSpace),
            }
        }
    }

    /// Consumes the cursor and returns the number of bytes written.
    pub fn finish(self) -> usize {
        self.written
    }

    /// Moves to write to the second underlying buffer, if there is at least
    /// `n` capacity. Otherwise, it means the top-level requested write is not
    /// successful, since we have reached the end of both buffers.
    #[inline]
    fn advance(&mut self, n: usize) -> Result<(), CursorError> {
        match &self.tail {
            Some(t) if t.len() >= n => {
                // Assume that remainder of head is now invalid
                self.written += self.head.len();
                self.head = self.tail.take().unwrap();
                Ok(())
            }
            _ => Err(CursorError::NoSpaceLeft),
        }
    }
}
