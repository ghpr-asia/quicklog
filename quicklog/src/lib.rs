//! Low-latency single-threaded logging library.
//!
//! # Usage
//!
//! `quicklog` provides an API similar to that of the `log` crate through the five logging macros: `trace!`, `debug!`, `info!`, `warn!` and `error!`.
//!
//! Note that the `init!()` macro needs to be called to initialize the logger before we can start logging.
//!
//! ## Example
//!
//! ```rust no_run
//! use quicklog::{init, flush, info};
//!
//! # fn main() {
//! // initialize required resources. by default, all logs are
//! // flushed to stdout.
//! init!();
//!
//! // basic usage
//! info!("Simple format string without arguments");
//! info!("Format string with arguments: {:?} {}", "hello world", 123);
//!
//! // structured fields -- follows similar rules to `tracing`.
//! info!(field_a = 123, field_b = "some text", "Structured fields: {:?}", 99);
//! info!(field_a = ?vec![1, 2, 3], field_b = %123, "Structured fields with sigils");
//!
//! // named parameters
//! let some_var = 10;
//! info!("Explicit capture of some_var: {some_var}", some_var = some_var);
//! info!("Implicit capture of some_var: {some_var}");
//!
//! // flushes everything in queue
//! while let Ok(()) = flush!() {}
//! # }
//! ```
//!
//! # Core: `Serialize`
//!
//! `quicklog` provides a `Serialize` trait which is used to opt into fast logging. For
//! convenience, a derive `Serialize` macro is provided to be used on relatively simple types
//! (similar to `Debug`). For more complicated user-defined types, a manual implementation of the
//! trait may be necessary.
//!
//! After implementing `Serialize` for user-defined types, there are two ways to enable `quicklog` to use them:
//!
//! 1. Place the argument before the format string, as part of the _structured fields_ (no prefix sigil is needed, unlike `?` and `%`). `quicklog` will automatically try to use the `Serialize` implementation for an argument placed in this position.
//!
//! 2. Use the `{:^}` formatting specifier in the format string, similar to how `{:?}` and `{}` are used for arguments implementing the `Debug` and `Display` traits respectively.
//!
//! ## Example
//! ```rust no_run
//! use quicklog::{flush, info, init, serialize::Serialize, Serialize};
//!
//! // derive `Serialize` macro
//! #[derive(Debug, Serialize)]
//! struct Foo {
//!     a: usize,
//!     b: String,
//!     c: Vec<&'static str>
//! }
//!
//! struct Bar {
//!     s: &'static str,
//! }
//!
//! impl Serialize for Bar {
//!     #[inline]
//!     fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> &'buf mut [u8] {
//!         self.s.encode(write_buf)
//!     }
//!
//!     fn decode(read_buf: &[u8]) -> (String, &[u8]) {
//!         let (output, rest) = <&str as Serialize>::decode(read_buf);
//!
//!         (format!("Bar {{ s: {} }}", output), rest)
//!     }
//!
//!     #[inline]
//!     fn buffer_size_required(&self) -> usize {
//!         self.s.buffer_size_required()
//!     }
//! }
//!
//! # fn main() {
//! let foo = Foo {
//!     a: 1000,
//!     b: "hello".to_string(),
//!     c: vec!["a", "b", "c"]
//! };
//! let bar = Bar {
//!     s: "hello world"
//! };
//! init!();
//!
//! // fast -- uses `Serialize`
//! info!(foo, "fast logging, using Serialize");
//! info!(bar, "fast logging, using Serialize");
//!
//! // structured field
//! info!(serialize_foo = foo, "fast logging, using Serialize");
//! info!(serialize_bar = bar, "fast logging, using Serialize");
//!
//! // format specifier
//! info!("fast logging, using Serialize: serialize_foo={:^}", foo);
//! info!("fast logging, using Serialize: serialize_bar={:^}", bar);
//!
//! // `quicklog` provides default implementations of `Serialize` for
//! // certain common data types
//! info!(a = 1, b = 2, c = "hello world".to_string(), "fast logging, using default Serialize");
//!
//! while let Ok(()) = flush!() {}
//! # }
//! ```
//!
//! #### Caveats
//!
//! Due to some constraints, mixing of `Serialize` and `Debug`/`Display` format specifiers in the format string is prohibited. For instance, this will fail to compile:
//!
//! ```rust compile_fail
//! # use quicklog::info;
//! // mixing {:^} with {:?} or {} not allowed!
//! # fn main() {
//! info!("hello world {:^} {:?} {}", 1, 2, 3);
//! # }
//! ```
//!
//! However, one can mix-and-match these arguments in the _structured fields_, for example:
//!
//! ```rust no_run
//! # use quicklog::info;
//! # #[derive(Debug, quicklog::Serialize)]
//! # struct Foo;
//! # fn main() {
//! # let some_serialize_struct = Foo;
//! # let some_debug_struct = Foo;
//! # let some_display_struct = 5;
//! info!(debug = ?some_debug_struct, display = %some_display_struct, serialize = some_serialize_struct, "serialize args in fmt str: {:^} {:^}", 3, 5);
//! # }
//! ```
//!
//! In general, for best performance, try to avoid mixing `Serialize` and non-`Serialize` arguments in each logging call. For instance, try to ensure that on performance-critical paths, every logging argument implements `Serialize`:
//!
//! ```rust no_run
//! # use quicklog::info;
//! # #[derive(quicklog::Serialize)]
//! # struct Foo;
//! # fn main() {
//! # let some_serialize_struct = Foo;
//! info!(a = 1, b = "hello world", c = 930.123, "Some message: {:^}", some_serialize_struct);
//! # }
//! ```
//!
//! # Advanced usage
//!
//! ## Customizing log output location and format
//!
//! Two interfaces are provided for configuring both the logging destination and output format.
//! They are the `Flush` and `PatternFormatter` traits respectively.
//!
//! ### `Flush`
//!
//! The [`Flush`] trait is exposed via the `quicklog-flush` crate and specifies a single log
//! destination. An implementor of `Flush` can be set as the default by passing it to the `with_flush!` macro after
//! calling `init!`.
//!
//! By default, logs are output to stdout via the provided [`StdoutFlusher`]. One can easily save logs to a file by using the provided [`FileFlusher`] instead.
//!
//! #### Example
//! ```rust no_run
//! use quicklog::{flush, info, init, with_flush_into_file, FileFlusher};
//!
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
//! ### `PatternFormatter`
//!
//! An implementor of [`PatternFormatter`] describes how the log line should be formatted. Apart
//! from the main logging message, information such as [`Metadata`] about the logging callsite and
//! the [`DateTime`](chrono::DateTime) are exposed through this trait. Similar to `Flush`, an
//! implementor of `PatternFormatter` can be set as the default by passing it to the
//! `with_formatter!` macro after calling `init!`.
//!
//! By default, logs have the `[utc datetime][log level]"message"` format:
//! ```rust no_run
//! use quicklog::{info, Serialize};
//!
//! #[derive(Serialize)]
//! struct S {
//!     i: usize,
//! }
//! # fn main() {
//! let some_struct = S { i: 0 };
//!
//! // [2023-11-29T05:05:39.310212084Z][INF]Some data: a=S { i: 0 }
//! info!(a = some_struct, "Some data:")
//! # }
//! ```
//!
//! #### Example
//! ```rust no_run
//! use quicklog::formatter::PatternFormatter;
//! use quicklog::queue::Metadata;
//! use quicklog::{DateTime, Utc};
//! use quicklog::{flush, init, info, with_flush_into_file, with_formatter};
//!
//! pub struct PlainFormatter;
//! impl PatternFormatter for PlainFormatter {
//!     fn custom_format(
//!         &mut self,
//!         _: DateTime<chrono::Utc>,
//!         _: &Metadata,
//!         _: &[String],
//!         log_record: &str,
//!     ) -> String {
//!         format!("{}\n", log_record)
//!     }
//! }
//!
//! # fn main() {
//! init!();
//!
//! // item goes into logging queue
//! info!("hello world");
//!
//! // flushed into stdout: [utc datetime]"hello world"
//! _ = flush!();
//!
//! // change log output format according to `PlainFormatter`
//! with_formatter!(PlainFormatter);
//! // flush into a file path specified
//! with_flush_into_file!("logs/my_log.log");
//!
//! info!("shave yaks");
//!
//! // flushed into file
//! _ = flush!();
//! # }
//! ```
//!
//! ## Compile-time log filtering
//!
//! There are two ways to filter out the generation and execution of logs:
//! 1. At compile-time
//!
//!    - This is done by setting the `QUICKLOG_MIN_LEVEL` environment variable which will be read during program compilation. For example, setting `QUICKLOG_MIN_LEVEL=ERR` will _generate_ the code for only `error`-level logs, while the other logs expand to nothing in the final output. Some accepted values for the environment variable include `INF`, `info`, `Info`, `2` for the info level, with similar syntax for the other levels as well.
//!
//! 2. At run-time
//!    - This uses a simple function, [`set_max_level`](quicklog/src/level.rs#L133), to set the maximum log level at runtime. This allows for more dynamic interleaving of logs, for example:
//! ```rust no_run
//! use quicklog::{error, info, init, level::{set_max_level, LevelFilter}};
//!
//! # fn main() {
//! init!();
//!
//! // log everything
//! set_max_level(LevelFilter::Trace);
//!
//! // recorded
//! info!("hello world");
//! // ...
//!
//! // only log errors from here on
//! set_max_level(LevelFilter::Error);
//! // this macro will be *expanded* during compilation, but not *executed*!
//! info!("hello world");
//! // recorded
//! error!("some error!");
//! # }
//! ```
//!
//! Note that compile-time filtering takes precedence over run-time filtering, since it influences whether `quicklog` will generate and expand the macro at build time in the first place. For instance, if we set `QUICKLOG_MIN_LEVEL=ERR`, then in the above program, the first `info("hello world")` will not be recorded at all. Also note that any filters set at runtime through `set_max_level` will have no effect if `QUICKLOG_MIN_LEVEL` is defined.
//!
//! Clearly, compile-time filtering is more restrictive than run-time filtering. However, performance-sensitive applications may still consider compile-time filtering since it avoids both a branch check and code generation for logs that one wants to filter out completely, which can have positive performance impact.
//!
//! ## JSON logging
//!
//! There are two ways to use built-in JSON logging:
//!
//! 1. `with_json_formatter!` macro to set [`JsonFormatter`] as the global default.
//!
//! ### Example
//!
//! ```rust no_run
//! use quicklog::{info, init, with_json_formatter};
//!
//! # fn main() {
//! init!();
//! with_json_formatter!();
//!
//! // {"timestamp":"2023-11-29T05:05:39.310212084Z","level":"INF","fields":{"message":"hello world, bye world","key1" = "123"}}
//! info!(key1 = 123, "hello world, {:?}", "bye world");
//! # }
//! ```
//!
//! 2. [`event!`] logging macro. This logs a *single message* in JSON format with a log level of
//!    `Level::Event`.
//!
//! ### Example
//!
//! ```rust no_run
//! use quicklog::{event, info, init};
//!
//! # fn main() {
//! init!();
//!
//! // {"timestamp":"2023-11-29T05:05:39.310212084Z","level":"EVT","fields":{"message":"hello world, bye world","key1" = "123"}}
//! event!(key1 = 123, "hello world, {:?}", "bye world");
//!
//! // normal, default format
//! // [2023-11-29T05:05:39.310212084Z][INF]hello world, bye world key1=123
//! info!(key1 = 123, "hello world, {:?}", "bye world");
//! # }
//! ```
//!
//! ## Deferred logging
//!
//! For more performance-sensitive applications, one can opt for the deferred logging macros: `trace_defer`, `debug_defer`, `info_defer`, `warn_defer` or `error_defer`. These macros accept the same logging syntax as their non-`defer` counterparts, but must be followed by an explicit call to `commit` in order for the logs to become visible via `flush`. This saves on a few potentially expensive atomic operations. This will most likely be useful when an application makes a series of logging calls consecutively in some kind of event loop, and only needs to flush/make visible those logs after the main events have been processed.
//! ```rust no_run
//! use quicklog::{commit, flush, info_defer, init};
//!
//! # fn main() {
//! init!();
//!
//! // log without making data visible immediately
//! info_defer!("hello world");
//! info_defer!(a = 1, b = 2, "some data");
//!
//! // no data committed yet!
//! assert!(flush!().is_err());
//!
//! // commits all data written so far
//! commit!();
//!
//! // output of logs should be visible now
//! while let Ok(()) = flush!() {}
//! # }
//! ```
//!
//! ## Configuration of max logging capacity
//!
//! The default size used for the backing queue used by `quicklog` is 1MB. To
//! specify a different size, pass the desired size to the [`init`] macro.
//! ```no_run
//! # use quicklog::{init, info};
//! # fn main() {
//! let sz = 10 * 1024 * 1024;
//!
//! // 10MB
//! init!(sz);
//!
//! let mut a = Vec::with_capacity(sz);
//! for i in 0..sz {
//!     a[i] = i;
//! }
//!
//! // log big struct using `Serialize`
//! info!(a, "inspecting some big data");
//! # }
//! ```
//!
//! Note that this size may be rounded up or adjusted for better performance.
//!
//! [`Serialize`]: serialize::Serialize
//! [`StdoutFlusher`]: crate::StdoutFlusher
//! [`FileFlusher`]: crate::FileFlusher
//! [`PatternFormatter`]: crate::formatter::PatternFormatter
//! [`JsonFormatter`]: crate::formatter::JsonFormatter
//! [`Metadata`]: crate::queue::Metadata
//! [`event!`]: crate::event

/// Formatters for structuring log output.
pub mod formatter;
/// Contains logging levels and filters.
pub mod level;
/// Macros for logging and modifying the currently used [`Flush`] handlers,
/// along with some utilities.
pub mod macros;
/// Operations and types involved with writing/reading to the global buffer.
pub mod queue;
/// [`Serialize`] trait for serialization of various data types to aid in
/// fast logging.
pub mod serialize;
/// Utility functions.
pub mod utils;

use bumpalo::Bump;
use dyn_fmt::AsStrFormatExt;
use formatter::{PatternFormatter, QuickLogFormatter};
use minstant::Instant;
use queue::{
    ArgsKind, Consumer, Cursor, FinishState, FlushError, FlushResult, LogArgType, LogHeader,
    Prepare, Producer, Queue, QueueError, ReadError, SerializePrepare, WriteFinish, WritePrepare,
    WriteState,
};
use serialize::DecodeFn;
use std::cell::OnceCell;

pub use ::bumpalo::collections::String as BumpString;

pub use chrono::{DateTime, Utc};

pub use quicklog_flush::{
    file_flusher::FileFlusher, noop_flusher::NoopFlusher, stdout_flusher::StdoutFlusher, Flush,
};
pub use quicklog_macros::{
    debug, debug_defer, error, error_defer, event, event_defer, info, info_defer, trace,
    trace_defer, warn, warn_defer, Serialize,
};

use crate::formatter::{construct_full_fmt_str, JsonFormatter};

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
        let mut decoded_args = Vec::new();
        match log_header.args_kind {
            ArgsKind::AllSerialize(decode_fn) => {
                cursor.read_decode_each(decode_fn, &mut decoded_args)?;
            }
            ArgsKind::Normal(num_args) => {
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
            }
        }

        let num_field_args = log_header.metadata.fields.len();
        debug_assert!(decoded_args.len() >= num_field_args);
        let end_idx = num_field_args.min(decoded_args.len());
        let field_start_idx = decoded_args.len() - end_idx;
        let field_args = &decoded_args[field_start_idx..];

        let log_line = if log_header.metadata.json {
            // Override global formatter
            let formatted = log_header
                .metadata
                .format_str
                .format(&decoded_args[..field_start_idx]);

            JsonFormatter.custom_format(time, log_header.metadata, field_args, &formatted)
        } else {
            let formatted = if self.formatter.include_structured_fields() {
                let fmt_str = construct_full_fmt_str(
                    log_header.metadata.format_str,
                    log_header.metadata.fields,
                );
                fmt_str.format(&decoded_args)
            } else {
                log_header
                    .metadata
                    .format_str
                    .format(&decoded_args[..field_start_idx])
            };

            self.formatter
                .custom_format(time, log_header.metadata, field_args, &formatted)
        };
        self.flusher.flush_one(log_line);

        let read = cursor.finish();
        self.receiver.finish_read(read);
        self.receiver.commit_read();

        Ok(())
    }

    /// Returns data needed in preparation for writing to the queue.
    #[inline]
    pub fn prepare_write_serialize(&mut self) -> WriteState<WritePrepare<'_, SerializePrepare>> {
        WriteState {
            state: WritePrepare {
                producer: &mut self.sender,
                prepare: SerializePrepare,
            },
        }
    }

    /// Returns data needed in preparation for writing to the queue.
    #[inline]
    pub fn prepare_write(&mut self) -> WriteState<WritePrepare<'_, Prepare<'_>>> {
        WriteState {
            state: WritePrepare {
                producer: &mut self.sender,
                prepare: Prepare {
                    fmt_buffer: &self.fmt_buffer,
                },
            },
        }
    }

    /// Marks write as complete and commits it for reading.
    #[inline]
    pub fn finish_and_commit<F: FinishState>(&mut self, write_state: WriteState<WriteFinish<F>>) {
        self.finish_write(write_state);
        self.commit_write();
    }

    /// Marks write as complete by advancing local writer.
    #[inline]
    pub fn finish_write<F: FinishState>(&mut self, write_state: WriteState<WriteFinish<F>>) {
        let n = write_state.state.written;
        write_state.state.finished.complete(&mut self.fmt_buffer);
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
