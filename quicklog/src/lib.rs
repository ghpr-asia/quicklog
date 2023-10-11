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
//! # use quicklog::{info, init, flush};
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
//! Quicklog allows a number of macros with 5 different levels of verbosity. These
//! wrap around [`log!`] with the corresponding levels.
//!
//! * [`trace!`]
//! * [`debug!`]
//! * [`info!`]
//! * [`warn!`]
//! * [`error!`]
//!
//! Internally, these shorthands call [`try_log!`] with their respective levels.
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
//! struct SomeStruct {
//!     num: i64
//! }
//!
//! impl Serialize for SomeStruct {
//!    fn encode(&self, write_buf: &'static mut [u8]) -> Store { /* some impl */ }
//!    fn buffer_size_required(&self) -> usize { /* some impl */ }
//! }
//!
//! fn main() {
//!     let s = SomeStruct { num: 1_000_000 };
//!     info!("some struct: {}", ^s);
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
//! info!("eager display {}; eager debug {}", %impl_display, ?impl_debug);
//! // logically expands into:
//! // info!(
//! //      "eager display {}; eager debug {}",
//! //      format!("{}",   impl_display),
//! //      format!("{:?}", impl_debug)
//! // );
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
//! info!("hello world {} {} {}", question.tricky = true, question.answer = ?value, question.val = &value);
//! // output: "hello world question.tricky=true question.answer=10 question.val=10"
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
//! # use quicklog::{init, flush, with_flush};
//! # use quicklog_flush::stdout_flusher::StdoutFlusher;
//! fn main() {
//!     init!();
//!
//!     with_flush!(StdoutFlusher);
//!
//!     // uses the StdoutFlusher passed in for flushing
//!     flush!();
//! }
//! ```
//!
//! [`Serialize`]: serialize::Serialize
//! [`StdoutFlusher`]: quicklog_flush::stdout_flusher::StdoutFlusher
//! [`FileFlusher`]: quicklog_flush::file_flusher::FileFlusher

use heapless::spsc::Queue;
use level::Level;
use once_cell::unsync::{Lazy, OnceCell};
use quanta::Instant;
use serialize::buffer::ByteBuffer;
use std::fmt::Display;

pub use std::{file, line, module_path};

use chrono::{DateTime, Utc};
use quicklog_clock::{quanta::QuantaClock, Clock};
use quicklog_flush::{file_flusher::FileFlusher, Flush};

/// re-export of crates, for use in macros
pub use lazy_format;
pub use paste;
pub use quicklog_flush;

/// contains logging levels and filters
pub mod level;
/// contains macros
pub mod macros;
/// contains trait for serialization and pre-generated impl for common types and buffer
pub mod serialize;

include!("constants.rs");
/// `constants.rs` is generated from `build.rs`, should not be modified manually
pub mod constants;

/// Internal API
///
/// timed log item being stored into logging queue
#[doc(hidden)]
pub type TimedLogRecord = (Instant, LogRecord);

/// Logger initialized to Quicklog
#[doc(hidden)]
static mut LOGGER: Lazy<Quicklog> = Lazy::new(Quicklog::default);

/// Producer side of queue
pub type Sender = heapless::spsc::Producer<'static, TimedLogRecord, MAX_LOGGER_CAPACITY>;
/// Result from pushing onto queue
pub type SendResult = Result<(), TimedLogRecord>;
/// Consumer side of queue
pub type Receiver = heapless::spsc::Consumer<'static, TimedLogRecord, MAX_LOGGER_CAPACITY>;
/// Result from trying to pop from logging queue
pub type RecvResult = Result<(), FlushError>;

/// Log is the base trait that Quicklog will implement.
/// Flushing and formatting is deferred while logging.
pub trait Log {
    /// Dequeues a single log record from logging queue and passes it to Flusher
    fn flush_one(&mut self) -> RecvResult;
    /// Enqueues a single log record onto logging queue
    fn log(&mut self, record: LogRecord) -> SendResult;
}

/// Errors that can be presented when flushing
#[derive(Debug)]
pub enum FlushError {
    /// Queue is empty
    Empty,
}

///  ha**Internal API**
///
/// Returns a mut reference to the globally static logger [`LOGGER`]
#[doc(hidden)]
pub fn logger() -> &'static mut Quicklog {
    unsafe { &mut LOGGER }
}

pub struct LogRecord {
    /// Level
    pub level: Level,
    /// Module path
    pub module_path: &'static str,
    /// File
    pub file: &'static str,
    /// Line
    pub line: u32,
    /// Log line captured by using LazyFormat which implements Display trait.
    pub log_line: Box<dyn Display>,
}

pub trait PatternFormatter {
    fn custom_format(&mut self, time: DateTime<Utc>, log_record: LogRecord) -> String;
}

pub struct QuickLogFormatter;

impl QuickLogFormatter {
    fn new() -> Self {
        Self {}
    }
}

impl PatternFormatter for QuickLogFormatter {
    fn custom_format(&mut self, time: DateTime<Utc>, object: LogRecord) -> String {
        format!("[{:?}]{}\n", time, object.log_line)
    }
}

/// Quicklog implements the Log trait, to provide logging
pub struct Quicklog {
    flusher: Box<dyn Flush>,
    clock: Box<dyn Clock>,
    formatter: Box<dyn PatternFormatter>,
    sender: OnceCell<Sender>,
    receiver: OnceCell<Receiver>,
    byte_buffer: ByteBuffer,
}

impl Quicklog {
    /// Sets which flusher to be used, used in [`with_flush!`]
    #[doc(hidden)]
    pub fn use_flush(&mut self, flush: Box<dyn Flush>) {
        self.flusher = flush
    }

    pub fn use_formatter(&mut self, formatter: Box<dyn PatternFormatter>) {
        self.formatter = formatter
    }

    /// Sets which flusher to be used, used in [`with_clock!`]
    #[doc(hidden)]
    pub fn use_clock(&mut self, clock: Box<dyn Clock>) {
        self.clock = clock
    }

    /// Initializes channel inside of quicklog, can be called
    /// through [`init!`] macro
    pub fn init(&mut self) {
        static mut QUEUE: Queue<TimedLogRecord, MAX_LOGGER_CAPACITY> = Queue::new();
        let (sender, receiver): (Sender, Receiver) = unsafe { QUEUE.split() };

        self.sender.set(sender).ok();
        self.receiver.set(receiver).ok();
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
}

impl Default for Quicklog {
    fn default() -> Self {
        Quicklog {
            flusher: Box::new(FileFlusher::new("logs/quicklog.log")),
            clock: Box::new(QuantaClock::new()),
            formatter: Box::new(QuickLogFormatter::new()),
            sender: OnceCell::new(),
            receiver: OnceCell::new(),
            byte_buffer: ByteBuffer::new(),
        }
    }
}

impl Log for Quicklog {
    fn log(&mut self, record: LogRecord) -> SendResult {
        match
            self.sender
                .get_mut()
                .expect("Sender is not initialized, `Quicklog::init()` needs to be called at the entry point of your application")
                .enqueue((self.clock.get_instant(), record))
        {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }

    fn flush_one(&mut self) -> RecvResult {
        match
            self.receiver
                    .get_mut()
                    .expect("RECEIVER is not initialized, `Quicklog::init()` needs to be called at the entry point of your application")
                    .dequeue()
        {
            Some((time_logged, record)) => {
                let log_line = self.formatter.custom_format(
                    self.clock
                        .compute_system_time_from_instant(time_logged)
                        .expect("Unable to get time from instant"),
                    record,
                );
                self.flusher.flush_one(log_line);
                Ok(())
            }
            None => Err(FlushError::Empty),
        }
    }
}
