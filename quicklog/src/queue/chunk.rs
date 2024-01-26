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
    use bumpalo::Bump;
    use core::mem::size_of;
    use std::fmt::{Arguments, Write};

    use crate::{
        serialize::{DecodeEachFn, DecodeFn, Serialize},
        utils::{any_as_bytes, try_split_at, unlikely},
        {FmtArgHeader, LogArgType, Producer, QueueError, ReadResult, SerializeArgHeader},
    };

    use super::ReadError;

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
            let (chunk, _) = try_split_at(buf, <Self as ChunkRead>::bytes_required())?;
            Ok(usize::from_le_bytes(chunk.try_into().unwrap()))
        }
    }

    impl ChunkRead for DecodeFn {
        fn read(buf: &[u8]) -> ReadResult<Self> {
            let (chunk, _) = try_split_at(buf, <Self as ChunkRead>::bytes_required())?;
            Ok(unsafe { std::mem::transmute(usize::from_le_bytes(chunk.try_into().unwrap())) })
        }
    }

    impl ChunkRead for DecodeEachFn {
        fn read(buf: &[u8]) -> ReadResult<Self> {
            let (chunk, _) = try_split_at(buf, <Self as ChunkRead>::bytes_required())?;
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

    struct RawBytes<'a>(&'a [u8]);

    /// Write implementation for a raw byte buffer. This differs from unwrapped
    /// &[u8] slices in that we don't write the length of the byte slice here.
    impl ChunkWrite for RawBytes<'_> {
        #[inline]
        fn write(&self, buf: &mut [u8]) -> usize {
            let n = self.0.len();
            debug_assert!(buf.len() >= n);

            // SAFETY: We requested the exact amount required from the queue, so
            // should not run out of space here.
            unsafe {
                buf.as_mut_ptr()
                    .copy_from_nonoverlapping(self.0.as_ptr(), n);
            }

            n
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
        /// Reconstructs a type implementing [`ChunkRead`] by reading through the
        /// underlying buffer(s).
        pub(crate) fn read<C: ChunkRead>(&mut self) -> ReadResult<C> {
            let required_size = C::bytes_required();
            let inner = self.inner.as_ref();
            let buf = {
                let len = self.pos.min(inner.len());
                &inner[len..]
            };
            if unlikely(buf.len() < required_size) {
                return Err(ReadError::insufficient_bytes());
            }

            let res = C::read(buf)?;
            self.pos += required_size;

            Ok(res)
        }

        /// Reads `n` bytes from the underlying buffer(s), and returns a slice if
        /// there is enough capacity.
        pub(crate) fn read_bytes(&mut self, n: usize) -> ReadResult<&[u8]> {
            let inner = self.inner.as_ref();
            let buf = {
                let len = self.pos.min(inner.len());
                &inner[len..]
            };
            if unlikely(buf.len() < n) {
                return Err(ReadError::insufficient_bytes());
            }
            let res = unsafe { buf.get_unchecked(..n) };
            self.pos += n;

            Ok(res)
        }

        pub(crate) fn read_decode_each(
            &mut self,
            decode_fn: DecodeEachFn,
            out: &mut Vec<String>,
        ) -> ReadResult<()> {
            let inner = self.inner.as_ref();
            let buf = {
                let len = self.pos.min(inner.len());
                &inner[len..]
            };
            let rest = decode_fn(buf, out)?;
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
            self.write(&RawBytes(formatted.as_bytes()));
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

    pub trait PrepareState {
        type ProgressType;
        fn progress(self) -> Self::ProgressType;
    }

    pub struct SerializePrepare;

    impl PrepareState for SerializePrepare {
        type ProgressType = SerializeProgress;

        #[inline]
        fn progress(self) -> Self::ProgressType {
            SerializeProgress
        }
    }

    pub struct Prepare<'write> {
        pub(crate) fmt_buffer: &'write Bump,
    }

    impl PrepareState for Prepare<'_> {
        type ProgressType = Progress;

        #[inline]
        fn progress(self) -> Self::ProgressType {
            Progress
        }
    }

    pub trait ProgressState {
        type FinishType;
        fn finish(self) -> Self::FinishType;
    }

    pub struct SerializeProgress;

    impl ProgressState for SerializeProgress {
        type FinishType = SerializeFinish;

        #[inline]
        fn finish(self) -> Self::FinishType {
            SerializeFinish
        }
    }

    pub struct Progress;

    impl ProgressState for Progress {
        type FinishType = Finish;

        #[inline]
        fn finish(self) -> Self::FinishType {
            Finish
        }
    }

    pub trait FinishState {
        #[allow(unused_variables)]
        fn complete(&self, fmt_buffer: &mut Bump) {}
    }

    pub struct SerializeFinish;
    impl FinishState for SerializeFinish {}

    pub struct Finish;
    impl FinishState for Finish {
        #[inline]
        fn complete(&self, fmt_buffer: &mut Bump) {
            fmt_buffer.reset();
        }
    }

    /// Preparation stage of writing to the queue.
    pub struct WritePrepare<'write, P> {
        pub(crate) producer: &'write mut Producer,
        pub(crate) prepare: P,
    }

    impl<'write, P: PrepareState> WriteState<WritePrepare<'write, P>> {
        /// Consumes self to signify start of write to queue -- all arguments should
        /// have been preprocessed (if required) and required sizes computed by this
        /// point.
        #[inline]
        pub fn start_write(
            self,
            n: usize,
        ) -> Result<WriteState<WriteInProgress<'write, P::ProgressType>>, QueueError> {
            let buffer = Cursor::new(self.state.producer.prepare_write(n)?);
            let progress = self.state.prepare.progress();

            Ok(WriteState {
                state: WriteInProgress { buffer, progress },
            })
        }
    }

    impl<'write> WriteState<WritePrepare<'write, Prepare<'write>>> {
        /// Allocates a formatted [`bumpalo`] string.
        #[inline]
        pub fn format_in(&mut self, args: Arguments) -> bumpalo::collections::String<'write> {
            let mut s =
                bumpalo::collections::String::with_capacity_in(2048, self.state.prepare.fmt_buffer);
            s.write_fmt(args).unwrap();
            s
        }
    }

    /// In the midst of writing to the queue.
    pub struct WriteInProgress<'write, P> {
        buffer: Cursor<&'write mut [u8]>,
        progress: P,
    }

    impl<'write> WriteState<WriteInProgress<'write, Progress>> {
        #[inline]
        pub fn write_serialize<T: Serialize>(&mut self, arg: &T) {
            self.state.buffer.write_serialize(arg);
        }

        #[inline]
        pub fn write_str(&mut self, s: impl AsRef<str>) {
            self.state.buffer.write_str(s);
        }
    }

    impl<'write, P: ProgressState> WriteState<WriteInProgress<'write, P>> {
        #[inline]
        pub fn write<T: ChunkWrite>(&mut self, arg: &T) {
            self.state.buffer.write(arg);
        }

        #[inline]
        pub fn finish(self) -> WriteState<WriteFinish<P::FinishType>> {
            WriteState {
                state: WriteFinish {
                    written: self.state.buffer.finish(),
                    finished: self.state.progress.finish(),
                },
            }
        }
    }

    /// Finished writing to the queue.
    pub struct WriteFinish<F> {
        pub(crate) written: usize,
        pub(crate) finished: F,
    }
}

#[doc(hidden)]
pub use __hidden::*;

use std::{array::TryFromSliceError, num::ParseIntError};

/// Result from reading from logging queue.
pub type ReadResult<T> = Result<T, ReadError>;

/// Error reading from the queue.
#[derive(Clone, Debug, PartialEq)]
pub struct ReadError(ReadErrorRepr);

impl std::error::Error for ReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0 as &dyn std::error::Error)
    }
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ReadError {
    /// Queue does not have sufficient valid bytes left to read the required
    /// value.
    #[inline]
    pub fn insufficient_bytes() -> Self {
        Self(ReadErrorRepr::InsufficientBytes)
    }

    /// Value parsed from the queue does not match expected value.
    #[inline]
    pub fn unexpected(got: impl ToString) -> Self {
        Self(ReadErrorRepr::UnexpectedValue {
            got: got.to_string(),
        })
    }
}

impl From<TryFromSliceError> for ReadError {
    fn from(_: TryFromSliceError) -> Self {
        Self::insufficient_bytes()
    }
}

impl From<ParseIntError> for ReadError {
    fn from(value: ParseIntError) -> Self {
        Self::unexpected(value.to_string())
    }
}

/// Error reading from the queue.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ReadErrorRepr {
    /// Queue does not have sufficient valid bytes left to read the required
    /// value.
    InsufficientBytes,
    /// Value parsed from the queue does not match expected value.
    UnexpectedValue { got: String },
}

impl std::error::Error for ReadErrorRepr {}

impl std::fmt::Display for ReadErrorRepr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InsufficientBytes => f.write_str("not enough bytes to parse this type"),
            Self::UnexpectedValue { got } => f.write_fmt(format_args!(
                "unexpected value encountered when parsing: {got}"
            )),
        }
    }
}
