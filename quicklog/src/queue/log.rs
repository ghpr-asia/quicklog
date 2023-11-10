use std::mem::size_of;

use quanta::Instant;

use crate::{level::Level, utils::any_as_bytes};

use super::{ChunkRead, ChunkWrite, ReadError, ReadResult};

/// Result from trying to pop from logging queue.
pub type FlushResult = Result<(), FlushError>;

/// Errors that can be presented when flushing.
#[derive(Debug)]
pub enum FlushError {
    /// Queue is empty
    Empty,
    /// Error while parsing arguments due to reaching queue end
    InsufficientSpace,
    /// Encountered parsing/conversion failure for expected types
    Decode,
}

/// Information related to each macro callsite.
#[derive(Debug, PartialEq)]
pub struct Metadata {
    pub module_path: &'static str,
    pub file: &'static str,
    pub line: u32,
    pub level: Level,
    pub format_str: &'static str,
    pub num_args: usize,
}

/// The type of logging argument.
///
/// `Fmt` typically refers to an argument whose [`Debug`](std::fmt::Debug)
/// or [`Display`](std::fmt::Display) implementation is used
/// instead, whereas `Serialize` refers to arguments whose
/// [`Serialize`](crate::serialize::Serialize) implementations are recorded.
#[derive(Clone, Copy, Debug)]
#[repr(usize)]
pub enum LogArgType {
    Fmt = 1,
    Serialize = 2,
}

impl TryFrom<usize> for LogArgType {
    type Error = ReadError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(LogArgType::Fmt),
            2 => Ok(LogArgType::Serialize),
            _ => Err(ReadError::UnexpectedValue),
        }
    }
}

impl ChunkRead for LogArgType {
    fn read(buf: &[u8]) -> ReadResult<Self> {
        usize::read(buf).unwrap().try_into()
    }

    fn bytes_required() -> usize {
        size_of::<Self>()
    }
}

/// Main header for every log record.
///
/// This is written at the start of every log record to inform the reader
/// how to read from the queue.
#[derive(Debug)]
#[repr(C)]
pub struct LogHeader<'a> {
    pub(crate) metadata: &'a Metadata,
    pub(crate) instant: Instant,
}

impl<'a> LogHeader<'a> {
    pub fn new(metadata: &'a Metadata, instant: Instant) -> Self {
        Self { metadata, instant }
    }
}

impl ChunkRead for LogHeader<'_> {
    fn read(buf: &[u8]) -> ReadResult<Self> {
        let (header_chunk, _) = buf.split_at(<Self as ChunkRead>::bytes_required());
        let (metadata_chunk, timestamp_chunk) = header_chunk.split_at(size_of::<&Metadata>());

        let metadata: &Metadata = unsafe {
            &*(usize::from_le_bytes(metadata_chunk.try_into().unwrap()) as *const Metadata)
        };
        let instant_bytes: [u8; size_of::<Instant>()] = timestamp_chunk.try_into().unwrap();
        let instant: Instant = unsafe { std::mem::transmute(instant_bytes) };

        Ok(LogHeader { metadata, instant })
    }

    fn bytes_required() -> usize {
        size_of::<Self>()
    }
}

impl ChunkWrite for LogHeader<'_> {
    fn write(&self, buf: &mut [u8]) -> usize {
        let (chunk, _) = buf.split_at_mut(self.bytes_required());
        chunk.copy_from_slice(any_as_bytes(self));

        chunk.len()
    }

    fn bytes_required(&self) -> usize {
        size_of::<Self>()
    }
}

/// Header for logging arguments using their
/// [`Serialize`](crate::serialize::Serialize) implementation.
#[derive(Debug)]
#[repr(C)]
pub struct SerializeArgHeader {
    pub type_of_arg: LogArgType,
    pub size_of_arg: usize,
    pub decode_fn: usize,
}

impl ChunkWrite for SerializeArgHeader {
    fn write(&self, buf: &mut [u8]) -> usize {
        let (chunk, _) = buf.split_at_mut(self.bytes_required());
        chunk.copy_from_slice(any_as_bytes(self));

        chunk.len()
    }

    fn bytes_required(&self) -> usize {
        size_of::<Self>()
    }
}

/// Header for logging arguments which are formatted into the buffer.
#[derive(Debug)]
#[repr(C)]
pub struct FmtArgHeader {
    pub type_of_arg: LogArgType,
    pub size_of_arg: usize,
}

impl ChunkWrite for FmtArgHeader {
    fn write(&self, buf: &mut [u8]) -> usize {
        let (chunk, _) = buf.split_at_mut(self.bytes_required());
        chunk.copy_from_slice(any_as_bytes(self));

        chunk.len()
    }

    fn bytes_required(&self) -> usize {
        size_of::<Self>()
    }
}
