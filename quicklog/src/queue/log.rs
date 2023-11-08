use std::mem::size_of;

use quanta::Instant;

use crate::{level::Level, utils::any_as_bytes};

use super::{ChunkRead, ChunkWrite, ReadError, ReadResult};

/// Result from trying to pop from logging queue
pub type FlushResult = Result<(), FlushError>;

/// Errors that can be presented when flushing
#[derive(Debug)]
pub enum FlushError {
    /// Queue is empty
    Empty,
    /// Error while parsing arguments due to reaching queue end
    InsufficientSpace,
    /// Encountered parsing/conversion failure for expected types
    Decode,
}

#[derive(Debug, PartialEq)]
pub struct Metadata {
    pub module_path: &'static str,
    pub file: &'static str,
    pub line: u32,
    pub level: Level,
    pub format_str: &'static str,
}

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

#[derive(Debug)]
#[repr(C)]
pub struct LogHeader<'a> {
    pub metadata: &'a Metadata,
    pub instant: Instant,
    pub num_args: usize,
}

impl ChunkRead for LogHeader<'_> {
    fn read(buf: &[u8]) -> ReadResult<Self> {
        let (header_chunk, _) = buf.split_at(<Self as ChunkRead>::bytes_required());
        let (metadata_chunk, header_rest) = header_chunk.split_at(size_of::<&Metadata>());
        let (timestamp_chunk, num_args_chunk) = header_rest.split_at(size_of::<Instant>());

        let metadata: &Metadata = unsafe {
            &*(usize::from_le_bytes(metadata_chunk.try_into().unwrap()) as *const Metadata)
        };
        let instant_bytes: [u8; size_of::<Instant>()] = timestamp_chunk.try_into().unwrap();
        let instant: Instant = unsafe { std::mem::transmute(instant_bytes) };
        let num_args = usize::from_le_bytes(num_args_chunk.try_into().unwrap());

        Ok(LogHeader {
            metadata,
            instant,
            num_args,
        })
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