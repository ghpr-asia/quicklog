use std::mem::size_of;

use crate::{level::Level, serialize::DecodeEachFn, utils::any_as_bytes, Instant};

use super::{ChunkRead, ChunkWrite, ReadError, ReadResult};

/// Result from trying to pop from logging queue.
pub type FlushResult = Result<(), FlushError>;

/// Errors that can be presented when flushing.
#[derive(Debug)]
pub enum FlushError {
    /// Queue is empty.
    Empty,
    /// Error while parsing arguments due to reaching queue end.
    InsufficientSpace,
    /// Encountered parsing/conversion failure for expected types.
    Decode,
}

/// The type of arguments found in the log record.
#[derive(Debug, PartialEq, Eq)]
#[repr(usize)]
#[repr(C)]
pub enum ArgsKind {
    /// All arguments implement [`Serialize`](crate::serialize::Serialize).
    ///
    /// This is the optimized case emitted by the logging macro, indicating that
    /// all the arguments have been packed into a single tuple argument (and
    /// should be unpacked accordingly on the receiving end).
    AllSerialize(DecodeEachFn) = 1,
    /// Mix of formatting arguments and arguments implementing
    /// [`Serialize`](crate::serialize::Serialize).
    ///
    /// Contains the number of arguments.
    Normal(usize) = 2,
}

/// Information related to each macro callsite.
#[derive(Debug, PartialEq)]
pub struct Metadata {
    pub module_path: &'static str,
    pub file: &'static str,
    pub line: u32,
    pub level: Level,
    pub format_str: &'static str,
}

impl Metadata {
    #[inline]
    pub const fn new(
        module_path: &'static str,
        file: &'static str,
        line: u32,
        level: Level,
        format_str: &'static str,
    ) -> Self {
        Self {
            module_path,
            file,
            line,
            level,
            format_str,
        }
    }
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
    pub(crate) args_kind: ArgsKind,
}

impl<'a> LogHeader<'a> {
    pub fn new(metadata: &'a Metadata, instant: Instant, args_kind: ArgsKind) -> Self {
        Self {
            metadata,
            instant,
            args_kind,
        }
    }
}

impl ChunkRead for LogHeader<'_> {
    fn read(buf: &[u8]) -> ReadResult<Self> {
        let (header_chunk, _) = buf.split_at(<Self as ChunkRead>::bytes_required());
        let (metadata_chunk, header_rest) = header_chunk.split_at(size_of::<&Metadata>());
        let (timestamp_chunk, args_kind_chunk) = header_rest.split_at(size_of::<Instant>());

        let metadata: &Metadata = unsafe {
            &*(usize::from_le_bytes(metadata_chunk.try_into().unwrap()) as *const Metadata)
        };
        let instant_bytes: [u8; size_of::<Instant>()] = timestamp_chunk.try_into().unwrap();
        let instant: Instant = unsafe { std::mem::transmute(instant_bytes) };

        let (args_kind_tag_chunk, args_kind_payload_chunk) =
            args_kind_chunk.split_at(size_of::<usize>());
        let args_kind = match usize::from_le_bytes(args_kind_tag_chunk.try_into().unwrap()) {
            1 => {
                let decode_fn = DecodeEachFn::read(args_kind_payload_chunk)?;

                ArgsKind::AllSerialize(decode_fn)
            }
            2 => {
                let num_args = usize::read(args_kind_payload_chunk)?;

                ArgsKind::Normal(num_args)
            }
            _ => return Err(ReadError::UnexpectedValue),
        };

        Ok(LogHeader {
            metadata,
            instant,
            args_kind,
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

/// Computes overall size required for writing a log record.
pub fn log_size_required(args: &[(LogArgType, usize)]) -> usize {
    let mut size_required = 0;
    size_required += size_of::<LogHeader>();

    for (arg_type, arg_size) in args {
        size_required += match arg_type {
            LogArgType::Fmt => size_of::<FmtArgHeader>(),
            LogArgType::Serialize => size_of::<SerializeArgHeader>(),
        };
        size_required += arg_size;
    }

    size_required
}
