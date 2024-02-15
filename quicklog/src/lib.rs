//! Low-latency single-threaded logging library.
//!
//! # Usage
//!
//! `quicklog` provides an API similar to that of the `log` crate through the five logging macros: [`trace!`], [`debug!`], [`info!`], [`warn!`] and [`error!`]. Log messages are encoded into a logging queue and decoded from the same queue when the user calls [`flush!`]. Note that messages are currently dropped if this queue is full (see [the below section](#configuration-of-max-logging-capacity) on how to adjust the queue capacity).
//!
//! Note that the [`init!`] macro should be called *once* to initialize the logger before we can start logging. A set of configuration options can be passed to it to override the defaults (see [`Config`] or [Advanced usage](#advanced-usage) for more details).
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
//! ### Caveats
//!
//! #### Types implementing `Copy`
//!
//! For convenience, `Serialize` is automatically implemented for all types implementing both
//! [`Copy`] and [`Debug`]. This means that something like the below example should work out of the
//! box (without needing to derive or manually implement `Serialize`):
//!
//! ```rust
//! # use quicklog::{config, flush, info, init, NoopFlusher};
//! #[derive(Copy, Clone, Debug)]
//! struct CopyStruct {
//!     a: usize,
//!     b: &'static str,
//!     c: i32,
//! }
//!
//! # fn main() {
//! init!(config().flusher(NoopFlusher));
//!
//! let a = CopyStruct {
//!     a: 0,
//!     b: "hello world",
//!     c: -5,
//! };
//! // since `CopyStruct` derives `Copy` and `Debug`, `Serialize` is automatically
//! // implemented
//! info!(my_copy_struct = a, "Logging Copy struct: {:^}", a);
//! assert!(flush!().is_ok());
//! # }
//! ```
//!
//! However, note that [references implement `Copy` as well](https://doc.rust-lang.org/core/marker/trait.Copy.html#impl-Copy-for-%26T). If a reference is passed as a logging argument, `quicklog` will copy the *reference*, not the *underlying data*. This can cause some problems if one is not careful. For instance, the following might cause Undefined Behavior:
//!
//! ```rust no_run
//! # use quicklog::{config, flush, info, init,  NoopFlusher};
//! #[derive(Copy, Clone, Debug)]
//! struct CopyStruct {
//!     a: usize,
//!     b: &'static str,
//!     c: i32,
//! }
//!
//! # fn main() {
//! init!(config().flusher(NoopFlusher));
//!
//! let a = CopyStruct {
//!     a: 0,
//!     b: "hello world",
//!     c: -5,
//! };
//!
//! // double reference; reference is taken within *expanded macro scope*.
//! // once the scope of the expanded `info!` ends, the written reference
//! // should not be considered valid anymore
//! info!(my_copy_struct = &&a);
//! assert!(flush!().is_ok());
//! # }
//! ```
//!
//! In the above example, the reference only exists *within the scope expanded by the `info!`
//! macro*, and as such is invalidated when we try to call `flush!`. Since all logging arguments are
//! already taken by reference, either pass the variable without a reference, or assign the
//! reference to a variable outside of the logging macro:
//!
//! ```rust
//! # use quicklog::{config, flush, info, init,  NoopFlusher};
//! # #[derive(Copy, Clone, Debug)]
//! # struct CopyStruct {
//! #     a: usize,
//! #     b: &'static str,
//! #     c: i32,
//! # }
//! # fn main() {
//! # init!(config().flusher(NoopFlusher));
//! # let a = CopyStruct {
//! #     a: 0,
//! #     b: "hello world",
//! #     c: -5,
//! # };
//! // pass variable by itself (taken by reference within the macro)
//! info!(my_copy_struct = a);
//! assert!(flush!().is_ok());
//!
//! // declare top-level reference
//! // NOTE: `flush!` must be called before the stack variable goes out of scope!
//! // Otherwise, we will encounter the same issue as with the Undefined Behavior
//! // example above.
//! let b = &&a;
//! info!(my_copy_struct = b);
//! assert!(flush!().is_ok());
//! # }
//! ```
//!
//! #### Mixing `Serialize` and `Debug`/`Display` format specifiers
//!
//! Due to some constraints, mixing of `Serialize` and `Debug`/`Display` format specifiers in the format string is prohibited. For instance, this will fail to compile:
//! ```rust compile_fail
//! # use quicklog::info;
//! // mixing {:^} with {:?} or {} not allowed!
//! # fn main() {
//! info!("hello world {:^} {:?} {}", 1, 2, 3);
//! # }
//! ```
//!
//! However, one can mix-and-match these arguments in the _structured fields_, for example:
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
//! This section covers some fairly advanced usage of `quicklog`. In general,
//! for various configuration options, look at [`Config`] for modifiable
//! settings. For other requirements, the below sections may cover your use
//! case.
//!
//! ## Customizing log output location and format
//!
//! Two interfaces are provided for configuring both the logging destination and output format.
//! They are the [`Flush`] and [`PatternFormatter`] traits respectively.
//!
//! ### `Flush`
//!
//! The [`Flush`] trait is exposed via the `quicklog-flush` crate and specifies a single log
//! destination. An implementor of `Flush` can be set as the default by passing it to
//! [`config`].
//!
//! By default, logs are output to stdout via the provided [`StdoutFlusher`]. To save logs to a
//! file instead, pass a filename to `config`:
//!
//! ```rust no_run
//! use quicklog::{config, flush, info, init, FileFlusher};
//!
//! # fn main() {
//! // by default, flushes to stdout via `StdoutFlusher`.
//! // here, we change the output location to a `quicklog.log` file
//! init!(config().file_flusher("quicklog.log"));
//!
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
//! implementor of `PatternFormatter` can be set as the default by passing it to `config`.
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
//! An example of defining a custom output format:
//!
//! ```rust no_run
//! use quicklog::fmt::{LogContext, PatternFormatter, Writer};
//! use quicklog::Metadata;
//! use quicklog::{DateTime, Utc};
//! use quicklog::{config, flush, init, info};
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
//! // change log output format according to `PlainFormatter`
//! // note that we can build an equivalent formatter using
//! // `formatter().without_time().with_level(false).build()` instead of
//! // defining this new `PlainFormatter`.
//! // also, flush into a file path specified
//! let config = config().formatter(PlainFormatter).file_flusher("logs/my_log.log");
//! init!(config);
//!
//! // only the message, "shave yaks" should be shown in the log output
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
//! ```rust no_run
//! use quicklog::{config, formatter, info, init};
//!
//! # fn main() {
//! // use JSON formatting
//! init!(config().formatter(formatter().json().build()));
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
//! specify a different size, configure the desired size through `config` and
//! pass the final configuration to the `init!` macro.
//! ```no_run
//! # use quicklog::{config, init, info};
//! # fn main() {
//! let sz = 10 * 1024 * 1024;
//!
//! // 10MB
//! init!(config().capacity(sz));
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
//! - `target-filter`: enables target-based filtering. When enabled, allows the use of
//! [`TargetFilter`] to filter out logs based on the logging target.
//!
//! [`Serialize`]: serialize::Serialize
//! [`Copy`]: std::marker::Copy
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
/// Contains target filters.
pub mod target;

use bumpalo::Bump;
use fmt::{FormatterBuilder, JsonFormatter, LogContext, PatternFormatter, Writer};
use level::{Level, LevelFilter};
use minstant::{Anchor, Instant};
use serialize::DecodeFn;
use std::cell::OnceCell;
use target::TargetFilter;

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

/// Settings to be passed to the logger.
///
/// Meant to be passed to the [`init!`] macro to setup the default logger.
///
/// # Examples
///
/// Configuring a custom [`PatternFormatter`](crate::fmt::PatternFormatter)
/// and flushing to a file:
///
/// ```rust
/// use quicklog::{config, formatter, init};
/// # fn main() {
/// let config = config()
///     .formatter(formatter().without_time().with_level(false).build())
///     .file_flusher("mylog.log");
/// init!(config);
/// # }
/// ```
pub struct Config {
    formatter: Box<dyn PatternFormatter>,
    flusher: Box<dyn Flush>,
    queue_capacity: usize,
    #[cfg(feature = "target-filter")]
    target_filter: Option<TargetFilter>,
}

impl Config {
    /// Used to amend which [`PatternFormatter`](crate::fmt::PatternFormatter)
    /// implementor is currently attached to the global
    /// [`Quicklog`](crate::Quicklog) logger.
    ///
    /// By default, logs are formatted with the format `[utc
    /// datetime][log level]"message`. See also the [top-level
    /// documentation](crate#patternformatter) for information on defining your own
    /// formatters.
    pub fn formatter<P: PatternFormatter + 'static>(self, p: P) -> Self {
        Self {
            formatter: Box::new(p),
            ..self
        }
    }

    /// Used to amend which [`Flush`](crate::Flush) implementor is
    /// currently attached to the global [`Quicklog`](crate::Quicklog) logger.
    ///
    /// By default, logs are flushed to stdout. See also the [top-level
    /// documentation](crate#flush) for information on defining your own flushers.
    pub fn flusher<F: Flush + 'static>(self, f: F) -> Self {
        Self {
            flusher: Box::new(f),
            ..self
        }
    }

    /// Overwrites the [`Flush`](crate::Flush)
    /// implementor in [`Quicklog`](crate::Quicklog) with a
    /// [`FileFlusher`](crate::FileFlusher) using the
    /// provided file path.
    ///
    /// By default, logs are flushed to stdout. See also the [top-level
    /// documentation](crate#flush) for information on defining your own flushers.
    pub fn file_flusher(self, s: &'static str) -> Self {
        Self {
            flusher: Box::new(FileFlusher::new(s)),
            ..self
        }
    }

    /// Modifies the capacity of the logging queue (default is 1MB).
    ///
    /// Note that this size may be rounded up or adjusted
    /// for better performance. See also the [top-level
    /// documentation](crate#configuration-of-max-logging-capacity).
    pub fn capacity(self, c: usize) -> Self {
        Self {
            queue_capacity: c,
            ..self
        }
    }

    /// Sets a [`TargetFilter`](crate::target::TargetFilter) on the global logger.
    ///
    /// This filters out logs at runtime based on their target and the log level
    /// filter attached to it. Note that the `target-filter` feature must be
    /// enabled for this to have any effect.
    pub fn target_filter(self, _target_filter: TargetFilter) -> Self {
        #[cfg(feature = "target-filter")]
        {
            Self {
                target_filter: Some(_target_filter),
                ..self
            }
        }

        #[cfg(not(feature = "target-filter"))]
        {
            eprintln!("Called `target_filter` but `target-filter` feature not enabled; this setting will be ignored.");
            self
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            formatter: Box::new(FormatterBuilder::default().build()),
            flusher: Box::new(StdoutFlusher),
            queue_capacity: MAX_LOGGER_CAPACITY,
            #[cfg(feature = "target-filter")]
            target_filter: None,
        }
    }
}

/// Initializes the default [`Config`] options.
pub fn config() -> Config {
    Config::default()
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
    #[cfg(feature = "target-filter")]
    target_filter: Option<TargetFilter>,
}

impl Quicklog {
    fn new(config: Config) -> Self {
        let (sender, receiver) = Queue::new(config.queue_capacity);
        let log_level = if cfg!(debug_assertions) {
            LevelFilter::Trace
        } else {
            LevelFilter::Info
        };
        let writer = Writer::default().with_flusher(config.flusher);

        Quicklog {
            writer,
            log_level,
            formatter: config.formatter,
            clock: Clock::default(),
            sender,
            receiver,
            fmt_buffer: Bump::with_capacity(MAX_FMT_BUFFER_CAPACITY),
            #[cfg(feature = "target-filter")]
            target_filter: config.target_filter,
        }
    }

    /// Eagerly initializes the global [`Quicklog`] logger.
    /// Can be called through [`init!`] macro.
    pub fn init() {
        unsafe {
            _ = LOGGER.get_or_init(|| Quicklog::new(Config::default()));
        }
    }

    /// Eagerly initializes the global [`Quicklog`] logger.
    /// Can be called through [`init!`] macro.
    pub fn init_with_config(config: Config) {
        unsafe {
            _ = LOGGER.get_or_init(|| Quicklog::new(config));
        }
    }

    /// Logs with a [`Level`] greater than or equal to the returned [`LevelFilter`]
    /// will be enabled, whereas the rest will be disabled.
    #[inline(always)]
    pub fn is_level_enabled(&self, level: Level) -> bool {
        self.log_level.is_enabled(level)
    }

    /// Logs are enabled in the following priority order:
    /// - If there is a [`LevelFilter`] set for the provided target, then we
    /// check against that.
    /// - Otherwise, fallback to the global (default) `LevelFilter`.
    #[inline(always)]
    pub fn is_enabled(&self, _target: &str, level: Level) -> bool {
        #[cfg(not(feature = "target-filter"))]
        {
            self.is_level_enabled(level)
        }

        #[cfg(feature = "target-filter")]
        {
            // Default to global level filter if overall target filter not set
            // or filter not set for this specific target
            let Some(target_level) = self
                .target_filter
                .as_ref()
                .and_then(|filter| filter.target_level(_target))
            else {
                return self.is_level_enabled(level);
            };

            target_level.is_enabled(level)
        }
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
        let fmt_res = if matches!(log_ctx.metadata().level(), Level::Event) {
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

    /// Helpers for implementing trait dispatch based on a priority order.
    ///
    /// This allows us to dispatch, statically, on specialized implementations
    /// of `Serialize`, depending on what trait bounds the input satifies.
    /// Currently, the priority order is Serialize > Copy + Display > Copy +
    /// Debug > Nothing (error thrown).
    /// This means that for any type `T`:
    /// - `T` has a custom/derived `Serialize` implementation: this is used, as expected.
    /// - Otherwise, if `T` satifies `Copy` and `Display`: we use a
    /// custom `Serialize` implementation by wrapping it in `CopyDisplay2Serialize`.
    /// - Otherwise, if `T` satifies `Copy` and `Debug`: we use a
    /// custom `Serialize` implementation by wrapping it in `CopyDbg2Serialize`.
    /// - Otherwise, `T` doesn't satisfy any bounds which we provide `Serialize` implementation
    /// for, and will throw an error.
    ///
    /// This priority is enforced through wrapping the input argument through
    /// `__wrap` with layers of references. The compiler will then dispatch on
    /// the first trait implemented for the wrapper type with the highest number
    /// of references, continuously dereferencing until it reaches a satisfied
    /// trait.
    mod dispatch {
        #[repr(transparent)]
        #[derive(Debug, PartialEq)]
        pub struct CopyDbg2Serialize<T>(pub T);

        #[repr(transparent)]
        #[derive(Debug, PartialEq)]
        pub struct CopyDisplay2Serialize<T>(pub T);

        pub struct S<T>(pub T);

        #[allow(unused)]
        pub struct NoWrapT;

        impl NoWrapT {
            #[allow(unused)]
            #[allow(clippy::new_ret_no_self)]
            #[inline(always)]
            pub fn new<T>(self, a: S<T>) -> T {
                a.0
            }
        }

        pub trait NoWrap {
            #[inline(always)]
            fn wrap(&self) -> NoWrapT {
                NoWrapT
            }
        }

        impl<T: crate::serialize::Serialize> NoWrap for &&&S<T> {}

        impl<T> NoWrap for S<T> {}

        #[allow(unused)]
        pub struct DisplayWrapT;

        impl DisplayWrapT {
            #[allow(unused)]
            #[allow(clippy::new_ret_no_self)]
            #[inline(always)]
            pub fn new<T>(self, a: S<T>) -> CopyDisplay2Serialize<T> {
                CopyDisplay2Serialize(a.0)
            }
        }

        pub trait DisplayWrap {
            #[inline(always)]
            fn wrap(&self) -> DisplayWrapT {
                DisplayWrapT
            }
        }

        impl<T: Copy + core::fmt::Display> DisplayWrap for &&S<T> {}

        #[allow(unused)]
        pub struct DebugWrapT;

        impl DebugWrapT {
            #[allow(unused)]
            #[allow(clippy::new_ret_no_self)]
            #[inline(always)]
            pub fn new<T>(self, a: S<T>) -> CopyDbg2Serialize<T> {
                CopyDbg2Serialize(a.0)
            }
        }

        pub trait DebugWrap {
            #[inline(always)]
            fn wrap(&self) -> DebugWrapT {
                DebugWrapT
            }
        }

        impl<T: Copy + core::fmt::Debug> DebugWrap for &S<T> {}

        #[doc(hidden)]
        #[macro_export]
        macro_rules! __wrap {
            ($e:expr) => {{
                #[allow(unused)]
                use $crate::__macro_helpers::{DebugWrap, DisplayWrap, NoWrap};
                (&&&&$crate::__macro_helpers::S($e))
                    .wrap()
                    .new($crate::__macro_helpers::S($e))
            }};
        }
    }

    pub use dispatch::*;

    #[cfg(test)]
    mod test {
        use crate::{self as quicklog, Serialize};

        #[test]
        fn wrap_copy() {
            #[derive(Copy, Clone, Debug)]
            struct CopyDebugStruct {
                _a: u32,
            }

            #[derive(Copy, Clone, Debug)]
            struct CopyDisplayStruct {
                _b: u32,
            }

            impl core::fmt::Display for CopyDisplayStruct {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{}", self._b)
                }
            }

            #[derive(Serialize, Copy, Clone, Debug, PartialEq)]
            struct SerializeStruct {
                _a: usize,
            }

            impl core::fmt::Display for SerializeStruct {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{}", self._a)
                }
            }

            // Serialize takes priority
            assert_eq!(crate::__wrap!(1_u32), 1);

            let s = SerializeStruct { _a: 0 };
            assert_eq!(crate::__wrap!(&s), &s);

            let a = CopyDebugStruct { _a: 0 };
            assert!(matches!(
                crate::__wrap!(&a),
                crate::__macro_helpers::CopyDbg2Serialize(_)
            ));

            // Display takes priority (over Debug)
            let b = CopyDisplayStruct { _b: 0 };
            assert!(matches!(
                crate::__wrap!(&b),
                crate::__macro_helpers::CopyDisplay2Serialize(_)
            ));
        }
    }
}
