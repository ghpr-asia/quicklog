//! An asynchronous single-threaded logger where formatting and I/O are deferred at callsite.
//!
//! # Overview
//!
//! `Quicklog` is provides a framework for logging where it allows for deferred
//! deferred formatting and deferred I/O of logging, which should in turn provide
//! more performant logging with low callsite latency.
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
//!     thread::spawn(|| {
//!         loop {
//!             flush!();
//!         }
//!     });
//! }
//! ```
//!
//! # Macros
//!
//! #### Shorthand Macros
//!
//! Quicklog allows a number of macros with 5 different levels of verbosity. They are:
//!
//! * [`trace!`]
//! * [`debug!`]
//! * [`info!`]
//! * [`warn!`]
//! * [`error!`]
//!
//! ## Setup Macros
//!
//! Quicklog allows a user specified [`Clock`] or [`Flush`] to be implemented by
//! the user. This can be passed in through these macros, as long as the
//! underlying struct implements the correct traits
//!
//! * [`with_clock!`]: Specify the Clock Quicklog uses
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
//! # use quicklog::{init, info, serialize::{Serialize, Store}};
//! struct SomeStruct {
//!     num: i64
//! }
//!
//! impl Serialize for SomeStruct {
//!    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> (Store<'buf>, &'buf mut[u8]) { /* some impl */ }
//!    fn decode(read_buf: &[u8]) -> (String, &[u8]) { /* some impl */ }
//!    fn buffer_size_required(&self) -> usize { /* some impl */ }
//! }
//!
//! fn main() {
//!     init!();
//!     let s = SomeStruct { num: 1_000_000 };
//!     info!(^s, "some struct:");
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
//! info!(question.answer = ?value, question.tricky=true, question.val= value, "some questions:");
//! // output: "some questions: question.tricky=true question.val=10 question.answer=10"
//! # }
//! ```
//!
//! # Environment variables
//!
//! There are two environment variables you can set:
//!
//! 1. `QUICKLOG_MAX_LOGGER_CAPACITY`
//!     - sets the size of the spsc ring buffer used for logging
//! 2. `QUICKLOG_MAX_SERIALIZE_BUFFER_CAPACITY`
//!     - sets the size of the byte buffer used for static serialization
//!     - this can be increased when you run into issues out of memory in debug
//!     when conducting load testing
//!
//! # Components
//!
//! ## quicklog-clock
//!
//! [`Clock`] is the trait for a clock that can be used with [`Quicklog`]. Clocks can
//! be swapped out at runtime with a different implementation.
//!
//! This swap should be done at the init stage of your application to ensure
//! that timings are consistent.
//!
//! ### Example
//!
//! ```ignore
//! struct SomeClock;
//!
//! impl Clock for SomeClock { /* impl methods */ }
//!
//! fn main() {
//!     init!();
//!
//!     with_clock!(SomeClock::new());
//!
//!     // logger now uses SomeClock for timestamping
//!     info!("Hello, world!");
//!     flush!();
//! }
//! ```
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
//! ```
//! # use quicklog::{info, init, flush, with_flush};
//! # use quicklog_flush::stdout_flusher::StdoutFlusher;
//! fn main() {
//!     init!();
//!
//!     with_flush!(StdoutFlusher);
//!     info!("hello world!");
//!
//!     // uses the StdoutFlusher passed in for flushing
//!     flush!();
//! }
//! ```
//!
//! [`Serialize`]: serialize::Serialize
//! [`StdoutFlusher`]: quicklog_flush::stdout_flusher::StdoutFlusher
//! [`FileFlusher`]: quicklog_flush::file_flusher::FileFlusher

use dyn_fmt::AsStrFormatExt;
use once_cell::unsync::Lazy;
use quanta::Instant;
use queue::{
    receiver::Receiver, sender::Sender, CursorRef, FlushError, FlushResult, LogArgType, Metadata,
    ReadError,
};
use rtrb::{chunks::WriteChunkUninit, RingBuffer};
use serialize::buffer::ByteBuffer;

pub use ::lazy_format;
pub use std::{file, line, module_path};

use chrono::{DateTime, Utc};
use quicklog_clock::{quanta::QuantaClock, Clock};
use quicklog_flush::{file_flusher::FileFlusher, Flush};

/// contains logging levels and filters
pub mod level;
/// contains macros
pub mod macros;
/// contains trait for serialization and pre-generated impl for common types and buffer
pub mod serialize;

include!("constants.rs");
/// `constants.rs` is generated from `build.rs`, should not be modified manually
pub mod constants;

pub mod queue;
mod utils;

pub use quicklog_macros::{debug, error, info, trace, warn, Serialize};

use crate::{queue::LogHeader, serialize::DecodeFn};

/// Logger initialized to Quicklog
#[doc(hidden)]
static mut LOGGER: Lazy<Quicklog> = Lazy::new(Quicklog::default);

/// **Internal API**
///
/// Returns a mut reference to the globally static logger [`LOGGER`]
#[doc(hidden)]
pub fn logger() -> &'static mut Quicklog {
    unsafe { &mut LOGGER }
}

pub trait PatternFormatter {
    fn custom_format(
        &mut self,
        time: DateTime<Utc>,
        metadata: &Metadata,
        log_record: &str,
    ) -> String;
}

pub struct QuickLogFormatter;

impl QuickLogFormatter {
    fn new() -> Self {
        Self {}
    }
}

impl PatternFormatter for QuickLogFormatter {
    fn custom_format(&mut self, time: DateTime<Utc>, _: &Metadata, log_record: &str) -> String {
        format!("[{:?}]{}\n", time, log_record)
    }
}

/// Main logging handler
pub struct Quicklog {
    flusher: Box<dyn Flush>,
    clock: Box<dyn Clock>,
    formatter: Box<dyn PatternFormatter>,
    sender: Sender,
    receiver: Receiver,
    fmt_buffer: String,
    byte_buffer: ByteBuffer,
}

impl Quicklog {
    /// Eagerly initializes the global [`Quicklog`] logger.
    /// Can be called through [`init!`] macro
    pub fn init() {
        // Referencing forces evaluation of Lazy
        _ = logger();
    }

    /// Sets which flusher to be used, used in [`with_flush!`]
    #[doc(hidden)]
    pub fn use_flush(&mut self, flush: Box<dyn Flush>) {
        self.flusher = flush
    }

    pub fn use_formatter(&mut self, formatter: Box<dyn PatternFormatter>) {
        self.formatter = formatter
    }

    /// Sets which clock to be used, used in [`with_clock!`]
    #[doc(hidden)]
    pub fn use_clock(&mut self, clock: Box<dyn Clock>) {
        self.clock = clock
    }

    /// Retrieves current [Instant](quanta::Instant).
    #[inline(always)]
    pub fn now(&self) -> Instant {
        self.clock.get_instant()
    }

    /// Internal API to get a chunk from buffer
    ///
    /// <strong>DANGER</strong>
    ///
    /// In release, the [`TAIL`] wraps around back to the start of the buffer when
    /// there isn't sufficient space left inside of [`BUFFER`]. If this happens,
    /// the buffer might overwrite previous data with anything.
    ///
    /// In debug, the method panics when we reach the end of the buffer
    #[doc(hidden)]
    pub fn get_chunk_as_mut(&mut self, chunk_size: usize) -> &mut [u8] {
        self.byte_buffer.get_chunk_as_mut(chunk_size)
    }

    /// Flushes all arguments from the queue.
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
        let chunk = self.receiver.read_chunk()?;
        let (head, tail) = chunk.as_slices();
        let mut cursor = CursorRef::new(head, tail);

        if cursor.is_empty() {
            return Err(FlushError::Empty);
        }

        loop {
            // Parse header for entire log message
            let Ok(log_header) = cursor.read::<LogHeader>() else {
                break;
            };

            let num_args = log_header.num_args;
            let mut decoded_args = Vec::with_capacity(num_args);
            while decoded_args.len() < num_args {
                let Ok(arg_type) = cursor.read::<LogArgType>() else {
                    break;
                };

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

            if decoded_args.len() != num_args {
                continue;
            }

            let time = self
                .clock
                .compute_system_time_from_instant(&log_header.instant)
                .expect("Unable to get time from Instant");
            let formatted = log_header.metadata.format_str.format(&decoded_args);
            let log_line = self
                .formatter
                .custom_format(time, log_header.metadata, &formatted);
            self.flusher.flush_one(log_line);
        }

        chunk.commit_all();

        Ok(())
    }

    /// Returns chunks/buffers for writing to
    pub fn prepare_write(&mut self) -> (WriteChunkUninit<'_, u8>, &mut String) {
        let chunk = self.sender.write_chunk().expect("queue full");
        let buf = &mut self.fmt_buffer;

        (chunk, buf)
    }

    /// Consumes a previously obtained write chunk and commits the written
    /// slots, making them available for reading
    pub fn finish_write(chunk: WriteChunkUninit<'_, u8>, committed: usize) {
        unsafe { chunk.commit(committed) }
    }
}

impl Default for Quicklog {
    fn default() -> Self {
        let (sender, receiver) = RingBuffer::<u8>::new(MAX_LOGGER_CAPACITY);

        Quicklog {
            flusher: Box::new(FileFlusher::new("logs/quicklog.log")),
            clock: Box::new(QuantaClock::new()),
            formatter: Box::new(QuickLogFormatter::new()),
            sender: Sender(sender),
            receiver: Receiver(receiver),
            fmt_buffer: String::with_capacity(2048),
            byte_buffer: ByteBuffer::new(),
        }
    }
}
