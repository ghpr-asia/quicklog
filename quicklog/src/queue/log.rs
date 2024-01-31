/// Types that are re-exported since they are used in macros.
///
/// Some types are used internally as well, especially during decoding, but they
/// are primarily exposed to be used downstream.
///
/// **WARNING: this is not a stable API!**
/// All code in this module is intended as part of the internal API of
/// `quicklog`. It is marked as public since it is used in the codegen for the
/// main logging macros. However, the code and API can change without warning in
/// any version update to `quicklog`. It is highly discouraged to rely on this
/// in any form.
#[doc(hidden)]
mod __hidden {
    use core::mem::size_of;

    use minstant::Instant;

    use crate::{
        serialize::DecodeEachFn,
        utils::try_split_at,
        {ChunkRead, ChunkWrite, ReadError, ReadResult},
    };

    use super::Metadata;

    /// The type of arguments found in the log record.
    #[derive(Debug, PartialEq, Eq)]
    #[repr(usize)]
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
                v => Err(ReadError::unexpected(v)),
            }
        }
    }

    impl ChunkRead for LogArgType {
        fn read(buf: &[u8]) -> ReadResult<Self> {
            usize::read(buf).unwrap().try_into()
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
        pub(crate) log_size: usize,
    }

    impl<'a> LogHeader<'a> {
        #[inline]
        pub fn new(
            metadata: &'a Metadata,
            instant: Instant,
            args_kind: ArgsKind,
            log_size: usize,
        ) -> Self {
            Self {
                metadata,
                instant,
                args_kind,
                log_size,
            }
        }
    }

    impl ChunkRead for LogHeader<'_> {
        fn read(buf: &[u8]) -> ReadResult<Self> {
            let (header_chunk, _) = try_split_at(buf, <Self as ChunkRead>::bytes_required())?;
            let (metadata_chunk, header_rest) = try_split_at(header_chunk, size_of::<&Metadata>())?;
            let (timestamp_chunk, header_rest) = try_split_at(header_rest, size_of::<Instant>())?;
            let (args_kind_chunk, log_size_chunk) =
                try_split_at(header_rest, size_of::<ArgsKind>())?;

            let metadata: &Metadata =
                unsafe { &*(usize::from_le_bytes(metadata_chunk.try_into()?) as *const Metadata) };
            let instant_bytes: [u8; size_of::<Instant>()] = timestamp_chunk.try_into()?;
            let instant: Instant = unsafe { std::mem::transmute(instant_bytes) };

            let (args_kind_tag_chunk, args_kind_payload_chunk) =
                try_split_at(args_kind_chunk, size_of::<usize>())?;
            let args_kind = match usize::from_le_bytes(args_kind_tag_chunk.try_into()?) {
                1 => {
                    let decode_fn = DecodeEachFn::read(args_kind_payload_chunk)?;

                    ArgsKind::AllSerialize(decode_fn)
                }
                2 => {
                    let num_args = usize::read(args_kind_payload_chunk)?;

                    ArgsKind::Normal(num_args)
                }
                v => return Err(ReadError::unexpected(v)),
            };

            let log_size = usize::from_le_bytes(log_size_chunk.try_into()?);

            Ok(LogHeader {
                metadata,
                instant,
                args_kind,
                log_size,
            })
        }
    }

    impl ChunkWrite for LogHeader<'_> {}

    /// Header for logging arguments using their
    /// [`Serialize`](crate::serialize::Serialize) implementation.
    #[derive(Debug)]
    #[repr(C)]
    pub(crate) struct SerializeArgHeader {
        pub(crate) type_of_arg: LogArgType,
        pub(crate) size_of_arg: usize,
        pub(crate) decode_fn: usize,
    }

    impl ChunkWrite for SerializeArgHeader {}

    /// Header for logging arguments which are formatted into the buffer.
    #[derive(Debug)]
    #[repr(C)]
    pub(crate) struct FmtArgHeader {
        pub(crate) type_of_arg: LogArgType,
        pub(crate) size_of_arg: usize,
    }

    impl ChunkWrite for FmtArgHeader {}

    #[inline]
    pub const fn log_header_size() -> usize {
        size_of::<LogHeader>()
    }

    /// Computes overall size required for writing a log record.
    #[inline]
    pub fn log_size_required(args: &[(LogArgType, usize)]) -> usize {
        let mut size_required = 0;
        size_required += log_header_size();

        for (arg_type, arg_size) in args {
            size_required += match arg_type {
                LogArgType::Fmt => size_of::<FmtArgHeader>(),
                LogArgType::Serialize => size_of::<SerializeArgHeader>(),
            };
            size_required += arg_size;
        }

        size_required
    }
}

#[doc(hidden)]
pub use __hidden::*;

use crate::{level::Level, ReadError};

/// Result from trying to pop from logging queue.
pub type FlushResult = Result<(), FlushError>;

pub(crate) type FlushReprResult = Result<(), FlushErrorRepr>;

/// Errors that can be presented when flushing.
#[derive(Debug, PartialEq)]
pub enum FlushError {
    /// Queue is empty.
    Empty,
    /// Failed to properly format log output.
    Formatting,
    /// Failure encountered when reading from queue. See also [`ReadError`](crate::ReadError).
    Read(ReadError),
}

impl std::error::Error for FlushError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Empty => None,
            Self::Formatting => None,
            Self::Read(e) => Some(e as &dyn std::error::Error),
        }
    }
}

impl std::fmt::Display for FlushError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => f.write_str("queue is empty"),
            Self::Formatting => f.write_str("failed to format proper log output"),
            Self::Read(_) => f.write_str("unexpected failure when reading from queue"),
        }
    }
}

impl From<std::fmt::Error> for FlushError {
    fn from(_: std::fmt::Error) -> Self {
        Self::Formatting
    }
}

impl From<ReadError> for FlushError {
    fn from(value: ReadError) -> Self {
        Self::Read(value)
    }
}

pub(crate) enum FlushErrorRepr {
    Empty,
    Formatting,
    Read { err: ReadError, log_size: usize },
}

impl FlushErrorRepr {
    pub(crate) fn read(err: ReadError, log_size: usize) -> Self {
        Self::Read { err, log_size }
    }
}

impl From<std::fmt::Error> for FlushErrorRepr {
    fn from(_: std::fmt::Error) -> Self {
        Self::Formatting
    }
}

/// Information about each logging event.
#[derive(Debug, PartialEq)]
pub struct Metadata {
    pub target: &'static str,
    pub file: &'static str,
    pub line: u32,
    pub level: Level,
    pub format_str: &'static str,
    pub fields: &'static [&'static str],
}

impl Metadata {
    #[inline]
    pub const fn new(
        target: &'static str,
        file: &'static str,
        line: u32,
        level: Level,
        format_str: &'static str,
        fields: &'static [&'static str],
    ) -> Self {
        Self {
            target,
            file,
            line,
            level,
            format_str,
            fields,
        }
    }
}
