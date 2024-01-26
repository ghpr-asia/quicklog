//! Low-latency single-threaded logging library.
//!
//! # Usage
//!
//! `quicklog` provides an API similar to that of the `log` crate through the five logging macros: [`trace!`], [`debug!`], [`info!`], [`warn!`] and [`error!`]. Log messages are encoded into a logging queue and decoded from the same queue when the user calls [`flush!`]. Note that messages are currently dropped if this queue is full (see [the below section](#configuration-of-max-logging-capacity) on how to adjust the queue capacity).
//!
//! Note that the [`init!`] macro needs to be called to initialize the logger before we can start logging.
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
//! The syntax accepted by the logging macros is similar to that of [`tracing`](https://docs.rs/tracing/latest/tracing/index.html#using-the-macros). However, do note that [`spans`](https://docs.rs/tracing/latest/tracing/index.html#spans) are not supported currently, and hence cannot be specified within the macros.
//!
//! # Core concept: `Serialize`
//!
//! `quicklog` provides a [`Serialize`] trait which is used to opt into fast logging. For
//! convenience, a derive `Serialize` macro is provided to be used on relatively simple types
//! (similar to [`Debug`]/[`Display`]). For more complicated user-defined types, a manual implementation of the
//! trait may be necessary.
//!
//! After implementing `Serialize` for user-defined types, there are two ways to enable `quicklog` to use them:
//!
//! 1. Place the argument before the format string, as part of the [_structured fields_](https://docs.rs/tracing/latest/tracing/#recording-fields) (no prefix sigil is needed, unlike `?` and `%`). `quicklog` will automatically try to use the `Serialize` implementation for an argument placed in this position.
//!
//! 2. Use the `{:^}` formatting specifier in the format string, similar to how `{:?}` and `{}` are used for arguments implementing the `Debug` and `Display` traits respectively.
//!
//! ## Example
//! ```rust no_run
//! use quicklog::{flush, info, init, serialize::Serialize, ReadResult, Serialize};
//!
//! // derive `Serialize` macro
//! #[derive(Serialize)]
//! struct Foo {
//!     a: usize,
//!     b: String,
//!     c: Vec<&'static str>
//! }
//!
//! struct StrWrapper(&'static str);
//!
//! struct Bar {
//!     s: StrWrapper,
//! }
//!
//! impl Serialize for Bar {
//!     #[inline]
//!     fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> &'buf mut [u8] {
//!         self.s.0.encode(write_buf)
//!     }
//!
//!     fn decode(read_buf: &[u8]) -> ReadResult<(String, &[u8])> {
//!         let (output, rest) = <&str as Serialize>::decode(read_buf)?;
//!
//!         Ok((format!("Bar {{ s: {} }}", output), rest))
//!     }
//!
//!     #[inline]
//!     fn buffer_size_required(&self) -> usize {
//!         self.s.0.buffer_size_required()
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
//!     s: StrWrapper("hello world")
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
//! They are the [`Flush`] and [`PatternFormatter`] traits respectively.
//!
//! ### `Flush`
//!
//! The [`Flush`] trait is exposed via the `quicklog-flush` crate and specifies a single log
//! destination. An implementor of `Flush` can be set as the default by passing it to the [`with_flush!`] macro after
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
//! // [1706065336][INF]Some data: a=S { i: 0 }
//! info!(a = some_struct, "Some data:")
//! # }
//! ```
//!
//! #### Example
//! ```rust no_run
//! use quicklog::fmt::{LogContext, PatternFormatter, Writer};
//! use quicklog::Metadata;
//! use quicklog::{DateTime, Utc};
//! use quicklog::{flush, init, info, with_flush_into_file, with_formatter};
//!
//! pub struct PlainFormatter;
//! impl PatternFormatter for PlainFormatter {
//!     fn custom_format(
//!         &self,
//!         ctx: LogContext<'_>,
//!         writer: &mut Writer,
//!     ) -> std::fmt::Result {
//!         use std::fmt::Write;
//!         writeln!(writer, "{}", ctx.full_message())
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
//! // note that we can achieve the same effect with
//! // `formatter().without_time().with_level(false).init()` instead of defining
//! // this new `PlainFormatter`
//! with_formatter!(PlainFormatter);
//!
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
//! ## Log filtering
//!
//! There are two ways to filter out the generation and execution of logs:
//! 1. At compile-time
//!
//!    - This is done by setting the `QUICKLOG_MIN_LEVEL` environment variable which will be read during program compilation. For example, setting `QUICKLOG_MIN_LEVEL=ERR` will _generate_ the code for only `error`-level logs, while the other logs expand to nothing in the final output. Some accepted values for the environment variable include `INF`, `info`, `Info`, `2` for the info level, with similar syntax for the other levels as well.
//!
//! 2. At run-time
//!    - By default, the log filter is set to `Trace` in Debug and `Info` in Release. This means that all logs with level `Trace` and above will be logged in Debug, whereas only logs with level `Info` and above will be logged in Release. See the documentation for [`Level`] for more information.
//!    - To modify this filter at runtime, the [`set_max_level`] function is provided. This allows for more dynamic interleaving of logs, for example:
//! ```rust no_run
//! use quicklog::{error, info, init, level::LevelFilter, set_max_level};
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
//! // `Info` logs have a lower level than `Error`, so this log will not be recorded.
//! // this macro will be *expanded* during compilation, but not *executed*!
//! info!("hello world");
//! // recorded
//! error!("some error!");
//! # }
//! ```
//!
//! Note that compile-time filtering takes precedence over run-time filtering, since it influences whether `quicklog` will generate and expand the macro at build time in the first place. For instance, if we set `QUICKLOG_MIN_LEVEL=ERR`, then in the above program, the first `info("hello world")` will not be recorded at all. Also note that any filters set at runtime through `set_max_level` will have no effect if `QUICKLOG_MIN_LEVEL` is defined.
//!
//! Clearly, compile-time filtering is more restrictive than run-time filtering. However, performance-sensitive applications may still consider compile-time filtering since it avoids both a branch check and code generation for logs that one wants to filter out completely, which can have positive performance impact. But as always, remember to profile and benchmark your application to see that it actually gives the results you want.
//!
//! ## JSON logging
//!
//! There are two ways to use built-in JSON logging:
//!
//! 1. Setup the default formatter using [`formatter()`] to use JSON representation:
//!
//! ### Example
//!
//! ```rust no_run
//! use quicklog::{formatter, info, init};
//!
//! # fn main() {
//! init!();
//! // use JSON formatting
//! formatter().json().init();
//!
//! // {"timestamp":"1706065336","level":"INF","fields":{"message":"hello world, bye world","key1" = "123"}}
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
//! // {"timestamp":"1706065336","level":"EVT","fields":{"message":"hello world, bye world","key1" = "123"}}
//! event!(key1 = 123, "hello world, {:?}", "bye world");
//!
//! // normal, default format
//! // [1706065336][INF]hello world, bye world key1=123
//! info!(key1 = 123, "hello world, {:?}", "bye world");
//! # }
//! ```
//!
//! ## Deferred logging
//!
//! For more performance-sensitive applications, one can opt for the deferred logging macros: [`trace_defer!`], [`debug_defer!`], [`info_defer!`], [`warn_defer!`] or [`error_defer!`]. These macros accept the same logging syntax as their non-`defer` counterparts, but must be followed by an explicit call to the [`commit!`] macro in order for the logs to become visible via `flush`. This saves on a few potentially expensive atomic operations. This will most likely be useful when an application makes a series of logging calls consecutively in some kind of event loop, and only needs to flush/make visible those logs after the main events have been processed.
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
//! A useful mental model would be to think of the normal logging macros (`info!`, `warn!`, etc) as
//! a call to their deferred equivalents, followed by an immediate call to `commit!`:
//!
//! ```rust no_run
//! # use quicklog::{commit, info, info_defer};
//! # fn main() {
//! info!("hello world!");
//!
//! // under the hood, effectively the same as
//! info_defer!("hello world!");
//! commit!();
//! # }
//! ```
//!
//! ### Caveats
//!
//! Note that the call to `commit!` must be reachable in order to guarantee that data written so
//! far is committed and becomes visible. This may not always be the case, for instance, when a
//! function exits early due to an error:
//!
//! ```rust no_run
//! use quicklog::{commit, info_defer};
//!
//! enum IntError {
//!     WrongInt
//! }
//!
//! fn possible_err(some_val: usize) -> Result<(), IntError> {
//!     info_defer!("Entered possible_err with value: {:^}", some_val);
//!
//!     // hot path: perform some computations
//!     // ...
//!
//!     // possible error path: function will exit without calling `commit!`
//!     if some_val < 5 {
//!         return Err(IntError::WrongInt);
//!     }
//!
//!     commit!();
//!     Ok(())
//! }
//! ```
//!
//! In this case, the log entry called by `info_defer!` at the start of the function will not be
//! immediately visible when the function exits until the next call to `commit!` or a non-`defer`
//! logging macro. Naturally, one could just insert another `commit!` call within the error branch
//! and things would be fine. Alternatively, if one doesn't care about seeing the results
//! immediately and `commit!` is called eventually after the function returns, then this is fine as
//! well.
//!
//! Otherwise, to guarantee that the results of a deferred log will be visible after the
//! function returns, regardless of which codepath it takes, use the [`commit_on_scope_end!`] macro. Using
//! the above example:
//! ```rust no_run
//! # use quicklog::{commit_on_scope_end, info_defer};
//! # enum IntError {
//! #     WrongInt
//! # }
//! fn possible_err(some_val: usize) -> Result<(), IntError> {
//!     info_defer!("Entered possible_err with value: {:^}", some_val);
//!     // will always call `commit!` when this function returns
//!     commit_on_scope_end!();
//!
//!     // hot path: perform some computations
//!     // ...
//!
//!     // possible error path: function will exit without calling `commit!`
//!     if some_val < 5 {
//!         return Err(IntError::WrongInt);
//!     }
//!
//!     // no longer need to call commit
//!     // commit!();
//!     Ok(())
//! }
//! ```
//!
//! Note that `commit_on_scope_end!` implicitly does the same thing as `commit!`, but *at the end of the
//! current scope*. So in most cases you would probably want to put it at the top-level/outermost scope
//! within a function.
//!
//! ## Configuration of max logging capacity
//!
//! As mentioned, log messages will be dropped if they are too big to be written
//! into the backing logging queue. To avoid this, one might consider increasing
//! the capacity of the queue.
//!
//! The default size used for the backing queue used by `quicklog` is 1MB. To
//! specify a different size, pass the desired size to the `init!` macro.
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
//! `quicklog` does not currently support unbounded logging (i.e. automatically
//! resizing of logging queue) or blocking when the queue is full. It is
//! advisable to ensure that either `flush!` is called regularly to avoid
//! accumulating lots of messages which might saturate the queue, or adjusting
//! the size of the queue during initialization to a safe limit.
//!
//! # Feature Flags
//!
//! The following feature flag(s) are available:
//!
//! - `ansi`: enables ANSI colors and formatting. When enabled, will toggle on ANSI colors in the
//! default formatter. See [`FormatterBuilder`] for configuration options. Disabled by default.
//!
//! [`Serialize`]: serialize::Serialize
//! [`Debug`]: std::fmt::Debug
//! [`Display`]: std::fmt::Display
//! [`StdoutFlusher`]: crate::StdoutFlusher
//! [`FileFlusher`]: crate::FileFlusher
//! [`PatternFormatter`]: crate::fmt::PatternFormatter
//! [`FormatterBuilder`]: crate::fmt::FormatterBuilder
//! [`JsonFormatter`]: crate::fmt::JsonFormatter
//! [`Metadata`]: crate::Metadata
//! [`event!`]: crate::event
//! [`commit!`]: crate::commit
//! [`format!`]: crate::fmt::format
//! [`commit_on_scope_end!`]: crate::commit_on_scope_end
//! [`trace_defer!`]: crate::trace_defer
//! [`debug_defer!`]: crate::debug_defer
//! [`info_defer!`]: crate::info_defer
//! [`warn_defer!`]: crate::warn_defer
//! [`error_defer!`]: crate::error_defer
//! [`trace!`]: crate::trace
//! [`debug!`]: crate::debug
//! [`info!`]: crate::info
//! [`warn!`]: crate::warn
//! [`error!`]: crate::error
//! [`init!`]: crate::init
//! [`with_flush!`]: crate::with_flush
//! [`set_max_level`]: crate::set_max_level
//! [`Level`]: crate::level::Level

/// Macros for logging and modifying the currently used [`Flush`] handlers,
/// along with some utilities.
mod macros;

/// Operations and types involved with writing/reading to the global buffer.
mod queue;

/// Utility functions.
mod utils;

/// Formatters for structuring log output.
pub mod fmt;
/// Contains logging levels and filters.
pub mod level;
/// [`Serialize`](crate::serialize::Serialize) trait for serialization of various data types to aid in
/// fast logging.
pub mod serialize;

use bumpalo::Bump;
use fmt::{FormatterBuilder, JsonFormatter, LogContext, PatternFormatter, Writer};
use level::{Level, LevelFilter};
use minstant::{Anchor, Instant};
use serialize::DecodeFn;
use std::cell::OnceCell;

use crate::queue::FlushErrorRepr;

pub use chrono::{DateTime, Utc};

pub use fmt::formatter;
pub use queue::*;

pub use quicklog_flush::{
    file_flusher::FileFlusher, noop_flusher::NoopFlusher, stdout_flusher::StdoutFlusher, Flush,
};
pub use quicklog_macros::{
    debug, debug_defer, error, error_defer, event, event_defer, info, info_defer, trace,
    trace_defer, warn, warn_defer, Serialize,
};

/// Logger initialized to [`Quicklog`].
#[doc(hidden)]
static mut LOGGER: OnceCell<Quicklog> = OnceCell::new();

const MAX_FMT_BUFFER_CAPACITY: usize = 1048576;
const MAX_LOGGER_CAPACITY: usize = 1048576;

/// Returns a mut reference to the globally static logger [`LOGGER`]
///
/// **WARNING: this is not a stable API!**
/// This piece of code is intended as part of the internal API of `quicklog`.
/// It is marked as public since it is used in the codegen for the main logging
/// macros. However, the code and API can change without warning in any version
/// update to `quicklog`. It is highly discouraged to rely on this in any form.
#[doc(hidden)]
#[inline(always)]
pub fn logger() -> &'static mut Quicklog {
    unsafe {
        LOGGER
            .get_mut()
            .expect("LOGGER not initialized, call `quicklog::init!()` first!")
    }
}

/// Modifies the maximum log level that will be logged.
///
/// If [`Level`] is greater than or equal to a [`LevelFilter`], then it is
/// enabled. See the documentation for [`Level`] for more details on what this
/// means, as well as the [crate documentation](crate#log-filtering) for an
/// example on how to use this function.
#[inline(always)]
pub fn set_max_level(level: LevelFilter) {
    logger().log_level = level;
}

#[derive(Default)]
pub(crate) struct Clock {
    anchor: Anchor,
}

impl Clock {
    fn compute_unix_nanos(&self, now: Instant) -> u64 {
        now.as_unix_nanos(&self.anchor)
    }
}

/// Main logging handler.
pub struct Quicklog {
    writer: Writer,
    log_level: LevelFilter,
    formatter: Box<dyn PatternFormatter>,
    clock: Clock,
    sender: Producer,
    receiver: Consumer,
    fmt_buffer: Bump,
}

impl Quicklog {
    fn new(logger_capacity: usize) -> Self {
        let (sender, receiver) = Queue::new(logger_capacity);
        let log_level = if cfg!(debug_assertions) {
            LevelFilter::Trace
        } else {
            LevelFilter::Info
        };

        Quicklog {
            writer: Writer::default(),
            log_level,
            formatter: Box::new(FormatterBuilder::default().build()),
            clock: Clock::default(),
            sender,
            receiver,
            fmt_buffer: Bump::with_capacity(MAX_FMT_BUFFER_CAPACITY),
        }
    }

    /// Logs with a [`Level`] greater than or equal to the returned [`LevelFilter`]
    /// will be enabled, whereas the rest will be disabled.
    #[inline(always)]
    pub fn is_level_enabled(&self, level: Level) -> bool {
        level as usize >= self.log_level as usize
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

    /// Sets which flusher to be used, used in [`with_flush!`].
    #[doc(hidden)]
    pub fn use_flush(&mut self, flush: Box<dyn Flush>) {
        self.writer.with_flusher(flush);
    }

    /// Sets which flusher to be used, used in [`with_formatter!`].
    #[doc(hidden)]
    pub fn use_formatter(&mut self, formatter: Box<dyn PatternFormatter>) {
        self.formatter = formatter;
    }

    fn flush_imp(&mut self) -> FlushReprResult {
        let chunk = self
            .receiver
            .prepare_read()
            .map_err(|_| FlushErrorRepr::Empty)?;
        let mut cursor = Cursor::new(chunk);

        // Parse header for entire log message
        // Note that if this fails, there is really nothing much we can do
        // internally.. except propagate the error back to the user to be
        // handled manually.
        let log_header = cursor
            .read::<LogHeader>()
            .map_err(|e| FlushErrorRepr::read(e, 0))?;
        let log_size = log_header.log_size;

        let propagate_err = |e: ReadError| FlushErrorRepr::read(e, log_size);

        let time = self.clock.compute_unix_nanos(log_header.instant);
        let mut decoded_args = Vec::new();
        match log_header.args_kind {
            ArgsKind::AllSerialize(decode_fn) => {
                cursor
                    .read_decode_each(decode_fn, &mut decoded_args)
                    .map_err(propagate_err)?;
            }
            ArgsKind::Normal(num_args) => {
                for _ in 0..num_args {
                    let arg_type = cursor.read::<LogArgType>().map_err(propagate_err)?;

                    let decoded = match arg_type {
                        LogArgType::Fmt => {
                            // Remaining: size of argument
                            let size_of_arg = cursor.read::<usize>().map_err(propagate_err)?;
                            let arg_chunk =
                                cursor.read_bytes(size_of_arg).map_err(propagate_err)?;

                            // Assuming that we wrote this using in-built std::fmt, so should be valid string
                            std::str::from_utf8(arg_chunk)
                                .map_err(|e| {
                                    propagate_err(ReadError::unexpected(format!(
                                        "{}; value: {:?}",
                                        e, arg_chunk
                                    )))
                                })?
                                .to_string()
                        }
                        LogArgType::Serialize => {
                            // Remaining: size of argument, DecodeFn
                            let size_of_arg = cursor.read::<usize>().map_err(propagate_err)?;
                            let decode_fn = cursor.read::<DecodeFn>().map_err(propagate_err)?;
                            let arg_chunk =
                                cursor.read_bytes(size_of_arg).map_err(propagate_err)?;

                            let (decoded, _) = decode_fn(arg_chunk).map_err(propagate_err)?;
                            decoded
                        }
                    };
                    decoded_args.push(decoded);
                }
            }
        }

        let log_ctx = LogContext::new(time, log_header.metadata, &decoded_args);
        let fmt_res = if matches!(log_ctx.metadata.level, Level::Event) {
            JsonFormatter::default().custom_format(log_ctx, &mut self.writer)
        } else {
            self.formatter.custom_format(log_ctx, &mut self.writer)
        };
        match fmt_res {
            Ok(()) => self.writer.flush(),
            Err(e) => {
                self.writer.clear();
                return Err(e.into());
            }
        }

        let read = cursor.finish();
        self.receiver.finish_read(read);
        self.receiver.commit_read();

        Ok(())
    }

    /// Flushes a single log record from the queue.
    ///
    /// Iteratively reads through the queue to extract encoded logging
    /// arguments. This happens by:
    /// 1. Checks for a log header, which provides information about the number
    /// of arguments to expect.
    /// 2. Parsing header-argument pairs.
    ///
    /// In the event of parsing failure, we try to skip over the current log
    /// (with the presumably correct log size).
    pub fn flush(&mut self) -> FlushResult {
        match self.flush_imp() {
            Ok(()) => Ok(()),
            Err(e) => {
                match e {
                    FlushErrorRepr::Empty => Err(FlushError::Empty),
                    FlushErrorRepr::Formatting => Err(FlushError::Formatting),
                    FlushErrorRepr::Read { err, log_size } => {
                        // Skip over the log that failed to parse correctly
                        self.receiver.finish_read(log_size);
                        self.receiver.commit_read();
                        Err(err.into())
                    }
                }
            }
        }
    }

    /// Helper function for benchmarks to quickly pretend all logs have been
    /// read and committed.
    #[doc(hidden)]
    #[cfg(feature = "bench")]
    pub fn flush_noop(&mut self) -> FlushResult {
        let chunk_len = {
            let chunk = self
                .receiver
                .prepare_read()
                .map_err(|_| FlushError::Empty)?;
            chunk.len()
        };
        self.receiver.finish_read(chunk_len);
        self.receiver.commit_read();

        Err(FlushError::Empty)
    }
}

/// **WARNING: this is not a stable API!**
/// This piece of code is intended as part of the internal API of `quicklog`.
/// It is marked as public since it is used in the codegen for the main logging
/// macros. However, the code and API can change without warning in any version
/// update to `quicklog`. It is highly discouraged to rely on this in any form.
#[doc(hidden)]
impl Quicklog {
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

/// Function wrapper that just calls the passed function, ensuring that it is
/// not expanded inline.
///
/// **WARNING: this is not a stable API!**
/// This piece of code is intended as part of the internal API of `quicklog`.
/// It is marked as public since it is used in the codegen for the main logging
/// macros. However, the code and API can change without warning in any version
/// update to `quicklog`. It is highly discouraged to rely on this in any form.
#[doc(hidden)]
#[inline(never)]
#[cold]
pub fn log_wrapper<F: FnOnce() -> Result<(), QueueError>>(f: F) -> Result<(), QueueError> {
    f()
}

/// Retrieves current timestamp (cycle count) using
/// [`Instant`](minstant::Instant).
///
/// **WARNING: this is not a stable API!**
/// This piece of code is intended as part of the internal API of `quicklog`.
/// It is marked as public since it is used in the codegen for the main logging
/// macros. However, the code and API can change without warning in any version
/// update to `quicklog`. It is highly discouraged to rely on this in any form.
#[doc(hidden)]
#[inline]
pub fn now() -> Instant {
    Instant::now()
}

/// Types/functions that are purely used in (support of) macros.
///
/// **WARNING: this is not a stable API!**
/// This piece of code is intended as part of the internal API of `quicklog`.
/// It is marked as public since it is used in the codegen for the main logging
/// macros. However, the code and API can change without warning in any version
/// update to `quicklog`. It is highly discouraged to rely on this in any form.
#[doc(hidden)]
pub mod __macro_helpers {
    pub struct CommitOnDrop;

    impl Drop for CommitOnDrop {
        #[inline(always)]
        fn drop(&mut self) {
            crate::logger().commit_write();
        }
    }
}
