//! An asynchronous single-threaded logger where formatting and I/O are deferred at callsite.
//!
//! # Overview
//!
//! `Quicklog` provides a framework for logging which allows for deferred
//! formatting and deferred I/O of logging, which should in turn provide more
//! performant logging with low callsite latency.
//!
//! ## Deferred Formatting
//!
//! #### Why?
//!
//! Formatting a `struct` into a `String` requires the overhead of serialization.
//! Deferring the serialization of a struct can be avoided by cloning / copying
//! the struct at a point in time, and saving that onto a queue.
//!
//! Later at the flush site, this struct is serialized into a string when I/O is
//! going to be performed.
//!
//! ## Deferred I/O
//!
//! #### Why?
//!
//! Deferring the I/O of formatting would allow for low callsite latency and allow
//! a user to implement their own flush site, possibly on a separate thread
//!
//! # Usage
//!
//! `init!()` macro needs to be called to initialize the logger before we can
//! start logging, probably near the entry point of your application.
//!
//! ## Example Usage
//!
//! ```
//! # use std::thread;
//! # use quicklog::{init, flush, info};
//! fn main() {
//!     init!();
//!
//!     // log some stuff
//!     info!("hello world! {}", "some argument");
//!
//!     // flush on separate thread
//!     thread::spawn(|| loop {
//!         flush!();
//!     });
//! }
//! ```
//!
//! The default size used for the backing queue used by `quicklog` is 1MB. To
//! specify a different size, pass the desired size to the [`init`] macro.
//!
//! ```no_run
//! # use quicklog::{init, info};
//! fn main() {
//!     let sz = 10 * 1024 * 1024;
//!
//!     // 10MB
//!     init!(sz);
//!
//!     let mut a = Vec::with_capacity(sz);
//!     for i in 0..sz {
//!         a[i] = i;
//!     }
//!
//!     // log big struct
//!     info!(a, "inspecting some big data");
//! }
//! ```
//!
//! Note that this size may be rounded up or adjusted for better performance.
//!
//! # Macros
//!
//! ### Shorthand Macros
//!
//! Quicklog allows a number of macros with 5 different levels of verbosity. They are:
//!
//! * [`trace!`]
//! * [`debug!`]
//! * [`info!`]
//! * [`warn!`]
//! * [`error!`]
//!
//! ### Example Usage
//!
//! ```ignore
//! // see section on macro prefixes for more information on prefixed arguments
//! info!(?debug_struct, %display_struct, serialize_struct, "hello world {:?}", debug_struct
//! );
//! ```
//!
//! See the repository examples for more advanced usage of the various syntax
//! patterns supported.
//!
//! ## Setup Macros
//!
//! Quicklog allows a user specified [`Flush`] to be implemented by the user.
//! This can be passed in through these macros, as long as the underlying struct
//! implements the correct traits
//!
//! * [`with_flush!`]: Specify the Flusher Quicklog uses
//! * [`with_flush_into_file`]: Specify path to flush log lines into
//!
//! ## Macro prefix for partial serialization
//!
//! To speed things up, if you are logging a large struct, there could be some small things
//! you might not want to log. This functionality can be done through implementing the
//! [`Serialize`] trait, where you can implement how to copy which parts of the struct.
//!
//! This could additionally be helpful if you already have the struct inside a buffer in byte
//! form, as you could simply pass the buffer directly into the decode fn, eliminiating any
//! need to copy.
//!
//! ```ignore
//! # use quicklog::{init, info, serialize::Serialize};
//! struct SomeStruct {
//!     num: i64
//! }
//!
//! impl Serialize for SomeStruct {
//!    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> &'buf mut[u8] { /* some impl */ }
//!    fn decode(read_buf: &[u8]) -> (String, &[u8]) { /* some impl */ }
//!    fn buffer_size_required(&self) -> usize { /* some impl */ }
//! }
//!
//! fn main() {
//!     init!();
//!     let s = SomeStruct { num: 1_000_000 };
//!     info!(s, "some struct:");
//! }
//! ```
//!
//! ## Macro prefix for eager evaluation
//!
//! There are two prefixes you can use for variables, `%` and `?`. This works the same
//! way as `tracing`, where `%` eagerly evaluates an object that implements `Display`
//! and `?` eagerly evaluates an object that implements `Debug`.
//!
//! ```
//! # use quicklog::{init, info};
//! # fn main() {
//! # let impl_debug = "";
//! # let impl_display = "";
//! # init!();
//! info!(%impl_display, ?impl_debug);
//!
//! // logically expands into:
//! // info!(format!("impl_display={}", impl_display), format!("impl_debug={:?}", impl_debug));
//! # }
//! ```
//!
//! ## Structured fields
//!
//! Structured fields in log lines can be specified using `field_name = field_value`
//! syntax. `field_name` can be a literal or a bunch of idents. This can also
//! be used in combination with `%` and `?` prefix on args to eagerly evaluate
//! expressions into format strings.
//!
//! ```
//! # use quicklog::{init, info};
//! # fn main() {
//! # init!();
//! # let value = 10;
//! info!(question.answer = ?value, question.tricky = "no", question.val = value, "some questions:");
//! // output: "some questions: question.tricky="no" question.val=10 question.answer=10"
//! # }
//! ```
//!
//! # Components
//!
//! ## quicklog-flush
//!
//! [`Flush`] is the trait that defines how the log messages would be flushed.
//! These logs can be printed through using the pre-defined [`StdoutFlusher`] or
//! saved to a file through the pre-defined [`FileFlusher`] to a specified
//! location through the string passed in.
//!
//! ### Example
//!
//! ```no_run
//! # use quicklog::{flush, info, init, with_flush_into_file, FileFlusher};
//! #
//! # fn main() {
//! init!();
//!
//! // by default, flushes to stdout via `StdoutFlusher`.
//! // here we change the output location to a `quicklog.log` file
//! with_flush_into_file!("quicklog.log");
//! info!("hello world!");
//!
//! // flushes to file
//! _ = flush!();
//! # }
//! ```
//!
//! [`Serialize`]: serialize::Serialize
//! [`StdoutFlusher`]: crate::StdoutFlusher
//! [`FileFlusher`]: crate::FileFlusher

/// Contains logging levels and filters.
pub mod level;
/// Macros for logging and modifying the currently used [`Flush`] handlers,
/// along with some utilities.
pub mod macros;
/// [`Serialize`] trait for serialization of various data types to aid in
/// fast logging.
pub mod serialize;

/// Operations and types involved with writing/reading to the global buffer.
pub mod queue;

/// Utility functions.
pub mod utils;

use bumpalo::Bump;
use dyn_fmt::AsStrFormatExt;
use minstant::Instant;
use queue::{
    ArgsKind, Consumer, Cursor, FlushError, FlushResult, LogArgType, LogHeader, Metadata, Producer,
    Queue, QueueError, ReadError,
};
use serialize::DecodeFn;
use std::cell::OnceCell;

pub use ::bumpalo::collections::String as BumpString;

pub use chrono::{DateTime, Utc};

pub use quicklog_flush::{
    file_flusher::FileFlusher, noop_flusher::NoopFlusher, stdout_flusher::StdoutFlusher, Flush,
};
pub use quicklog_macros::{
    debug, debug_defer, error, error_defer, info, info_defer, trace, trace_defer, warn, warn_defer,
    Serialize,
};

/// Logger initialized to [`Quicklog`].
#[doc(hidden)]
static mut LOGGER: OnceCell<Quicklog> = OnceCell::new();

const MAX_FMT_BUFFER_CAPACITY: usize = 1048576;
const MAX_LOGGER_CAPACITY: usize = 1048576;

/// **Internal API**
///
/// Returns a mut reference to the globally static logger [`LOGGER`]
#[doc(hidden)]
#[inline(always)]
pub fn logger() -> &'static mut Quicklog {
    unsafe {
        LOGGER
            .get_mut()
            .expect("LOGGER not initialized, call `quicklog::init!()` first!")
    }
}

/// Specifies how to format the final log record.
pub trait PatternFormatter {
    /// Customize format output as desired.
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::{DateTime, Utc};
    /// use quicklog::{init, queue::Metadata, with_formatter, PatternFormatter};
    ///
    /// struct MyFormatter {
    ///     callsite: &'static str,
    /// }
    ///
    /// impl PatternFormatter for MyFormatter {
    ///     fn custom_format(
    ///         &mut self,
    ///         time: DateTime<Utc>,
    ///         metadata: &Metadata,
    ///         log_record: &str,
    ///     ) -> String {
    ///         format!(
    ///             "[CALLSITE: {}][{:?}][{}]{}\n",
    ///             self.callsite, time, metadata.level, log_record
    ///         )
    ///     }
    /// }
    ///
    /// # fn main() {
    /// init!();
    /// let my_formatter = MyFormatter {
    ///     callsite: "main callsite",
    /// };
    /// with_formatter!(my_formatter);
    /// // logging calls...
    /// # }
    /// ```
    fn custom_format(
        &mut self,
        time: DateTime<Utc>,
        metadata: &Metadata,
        log_record: &str,
    ) -> String;
}

/// A basic formatter implementing [`PatternFormatter`].
pub struct QuickLogFormatter;

impl PatternFormatter for QuickLogFormatter {
    fn custom_format(
        &mut self,
        time: DateTime<Utc>,
        metadata: &Metadata,
        log_record: &str,
    ) -> String {
        format!("[{:?}][{}]{}\n", time, metadata.level, log_record)
    }
}

pub(crate) struct Clock {
    anchor_time: DateTime<Utc>,
    anchor_instant: Instant,
}

impl Clock {
    fn compute_datetime(&self, now: Instant) -> DateTime<Utc> {
        let duration = now - self.anchor_instant;
        self.anchor_time + duration
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            anchor_time: Utc::now(),
            anchor_instant: Instant::now(),
        }
    }
}

/// Contains data needed in preparation for writing to the queue.
pub struct WriteStart<'write> {
    producer: &'write mut Producer,
    pub fmt_buffer: &'write Bump,
}

impl<'write> WriteStart<'write> {
    /// Consumes self to signify start of write to queue -- all arguments should
    /// have been preprocessed (if required) and required sizes computed by this
    /// point.
    #[inline]
    pub fn start_write(self, n: usize) -> Result<&'write mut [u8], QueueError> {
        self.producer.prepare_write(n)
    }
}

/// Main logging handler.
pub struct Quicklog {
    flusher: Box<dyn Flush>,
    formatter: Box<dyn PatternFormatter>,
    clock: Clock,
    sender: Producer,
    receiver: Consumer,
    fmt_buffer: Bump,
}

impl Quicklog {
    fn new(logger_capacity: usize) -> Self {
        let (sender, receiver) = Queue::new(logger_capacity);

        Quicklog {
            flusher: Box::new(StdoutFlusher),
            formatter: Box::new(QuickLogFormatter),
            clock: Clock::default(),
            sender,
            receiver,
            fmt_buffer: Bump::with_capacity(MAX_FMT_BUFFER_CAPACITY),
        }
    }

    /// Eagerly initializes the global [`Quicklog`] logger.
    /// Can be called through [`init!`] macro.
    pub fn init() {
        unsafe {
            _ = LOGGER.get_or_init(|| Quicklog::new(MAX_LOGGER_CAPACITY));
        }
    }

    /// Eagerly initializes the global [`Quicklog`] logger.
    /// Can be called through [`init!`] macro.
    pub fn init_with_capacity(capacity: usize) {
        unsafe {
            _ = LOGGER.get_or_init(|| Quicklog::new(capacity));
        }
    }

    /// Retrieves current timestamp (cycle count) using
    /// [`Instant`](minstant::Instant).
    #[inline]
    pub fn now() -> Instant {
        Instant::now()
    }

    /// Sets which flusher to be used, used in [`with_flush!`].
    #[doc(hidden)]
    pub fn use_flush(&mut self, flush: Box<dyn Flush>) {
        self.flusher = flush
    }

    /// Sets which flusher to be used, used in [`with_formatter!`].
    pub fn use_formatter(&mut self, formatter: Box<dyn PatternFormatter>) {
        self.formatter = formatter
    }

    /// Flushes a single log record from the queue.
    ///
    /// Iteratively reads through the queue to extract encoded logging
    /// arguments. This happens by:
    /// 1. Checking for a [`LogHeader`], which provides information about
    ///    the number of arguments to expect.
    /// 2. Parsing header-argument pairs.
    ///
    /// In the event of parsing failure, the flushing is terminated without
    /// committing.
    pub fn flush(&mut self) -> FlushResult {
        let chunk = self
            .receiver
            .prepare_read()
            .map_err(|_| FlushError::Empty)?;
        let mut cursor = Cursor::new(chunk);

        // Parse header for entire log message
        let log_header = cursor.read::<LogHeader>()?;

        let time = self.clock.compute_datetime(log_header.instant);
        let formatted = match log_header.args_kind {
            ArgsKind::AllSerialize(decode_fn) => {
                let mut decoded_args = Vec::new();
                cursor.read_decode_each(decode_fn, &mut decoded_args)?;

                log_header.metadata.format_str.format(&decoded_args)
            }
            ArgsKind::Normal(num_args) => {
                let mut decoded_args = Vec::with_capacity(num_args);
                for _ in 0..num_args {
                    let arg_type = cursor.read::<LogArgType>()?;

                    let decoded = match arg_type {
                        LogArgType::Fmt => {
                            // Remaining: size of argument
                            let size_of_arg = cursor.read::<usize>()?;
                            let arg_chunk = cursor.read_bytes(size_of_arg)?;

                            // Assuming that we wrote this using in-built std::fmt, so should be valid string
                            std::str::from_utf8(arg_chunk)
                                .map_err(|_| ReadError::UnexpectedValue)?
                                .to_string()
                        }
                        LogArgType::Serialize => {
                            // Remaining: size of argument, DecodeFn
                            let size_of_arg = cursor.read::<usize>()?;
                            let decode_fn = cursor.read::<DecodeFn>()?;
                            let arg_chunk = cursor.read_bytes(size_of_arg)?;

                            let (decoded, _) = decode_fn(arg_chunk);
                            decoded
                        }
                    };
                    decoded_args.push(decoded);
                }

                log_header.metadata.format_str.format(&decoded_args)
            }
        };

        let log_line = self
            .formatter
            .custom_format(time, log_header.metadata, &formatted);
        self.flusher.flush_one(log_line);

        let read = cursor.finish();
        self.receiver.finish_read(read);
        self.receiver.commit_read();

        Ok(())
    }

    /// Returns data needed in preparation for writing to the queue.
    #[inline]
    pub fn prepare_write(&mut self) -> WriteStart<'_> {
        WriteStart {
            producer: &mut self.sender,
            fmt_buffer: &self.fmt_buffer,
        }
    }

    /// Marks write as complete and commits it for reading.
    #[inline]
    pub fn finish_and_commit(&mut self, n: usize) {
        self.finish_write(n);
        self.commit_write();
    }

    /// Marks write as complete by advancing local writer.
    #[inline]
    pub fn finish_write(&mut self, n: usize) {
        self.fmt_buffer.reset();
        self.sender.finish_write(n);
    }

    /// Commits all uncommitted writes to make slots available for reading.
    #[inline]
    pub fn commit_write(&mut self) {
        self.sender.commit_write();
    }
}

/// **WARNING: this is part of the public API and is primarily to aid in macro
/// codegen.**
///
/// Function wrapper that just calls the passed function, ensuring that it is
/// not expanded inline.
#[inline(never)]
#[cold]
pub fn log_wrapper<F: FnOnce() -> Result<(), QueueError>>(f: F) -> Result<(), QueueError> {
    f()
}
