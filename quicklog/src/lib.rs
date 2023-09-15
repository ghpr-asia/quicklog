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
use once_cell::unsync::Lazy;
use quanta::Instant;
use serialize::buffer::{Buffer, BUFFER};
use std::{cell::OnceCell, fmt::Display};

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

/// Sender handles sending into the mpsc queue
static mut SENDER: OnceCell<Sender> = OnceCell::new();
/// Receiver handles flushing from mpsc queue
static mut RECEIVER: OnceCell<Receiver> = OnceCell::new();

/// Log is the base trait that Quicklog will implement.
/// Flushing and formatting is deferred while logging.
pub trait Log {
    /// Dequeues a single log record from logging queue and passes it to Flusher
    fn flush_one(&mut self) -> RecvResult;
    /// Enqueues a single log record onto logging queue
    fn log(&self, record: LogRecord) -> SendResult;
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
    pub fn init() {
        Quicklog::init_channel();
        Quicklog::init_buffer();
    }

    /// Initializes buffer for static serialization
    fn init_buffer() {
        unsafe {
            BUFFER.set(Buffer::new()).ok();
        }
    }

    /// Initializes channel for main logging queue
    fn init_channel() {
        static mut QUEUE: Queue<TimedLogRecord, MAX_LOGGER_CAPACITY> = Queue::new();
        let (sender, receiver): (Sender, Receiver) = unsafe { QUEUE.split() };
        unsafe {
            SENDER.set(sender).ok();
            RECEIVER.set(receiver).ok();
        }
    }
}

impl Default for Quicklog {
    fn default() -> Self {
        Quicklog {
            flusher: Box::new(FileFlusher::new("logs/quicklog.log")),
            clock: Box::new(QuantaClock::new()),
            formatter: Box::new(QuickLogFormatter::new()),
        }
    }
}

impl Log for Quicklog {
    fn log(&self, record: LogRecord) -> SendResult {
        match unsafe {
            SENDER
                .get_mut()
                .expect("Sender is not initialized, `Quicklog::init()` needs to be called at the entry point of your application")
                .enqueue((self.clock.get_instant(), record))
        } {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }

    fn flush_one(&mut self) -> RecvResult {
        match unsafe {
            RECEIVER
                    .get_mut()
                    .expect("RECEIVER is not initialized, `Quicklog::init()` needs to be called at the entry point of your application")
                    .dequeue()
        } {
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

#[cfg(test)]
mod tests {
    use std::{str::from_utf8, sync::Mutex};

    use chrono::{DateTime, Utc};
    use quicklog_flush::Flush;

    use crate::{
        debug, error, flush, info,
        serialize::{Serialize, Store},
        trace, warn, LogRecord, PatternFormatter,
    };

    pub struct TestFormatter;

    impl TestFormatter {
        fn new() -> Self {
            Self {}
        }
    }

    impl PatternFormatter for TestFormatter {
        fn custom_format(&mut self, time: DateTime<Utc>, log_record: LogRecord) -> String {
            format!(
                "[{:?}][{}]\t{}\n",
                time, log_record.level, log_record.log_line
            )
        }
    }

    struct VecFlusher {
        pub vec: &'static mut Vec<String>,
    }

    impl VecFlusher {
        pub fn new(vec: &'static mut Vec<String>) -> VecFlusher {
            VecFlusher { vec }
        }
    }

    impl Flush for VecFlusher {
        fn flush_one(&mut self, display: String) {
            self.vec.push(display);
        }
    }

    #[derive(Clone, Debug)]
    struct Something {
        some_str: &'static str,
    }

    fn message_from_log_line(log_line: &str) -> String {
        log_line
            .split('\t')
            .last()
            .map(|s| s.chars().take(s.len() - 1).collect::<String>())
            .unwrap()
    }

    fn message_and_level_from_log_line(log_line: &str) -> String {
        let timestamp_end_idx = log_line.find(']').unwrap() + 1;
        log_line
            .chars()
            .skip(timestamp_end_idx)
            .take(log_line.len() - timestamp_end_idx - 1)
            .collect::<String>()
    }

    /// tests need to be single threaded, this mutex ensures
    /// tests are only executed in single threaded mode
    /// and [`Quicklog::init!`] is only called once.
    static TEST_LOCK: Mutex<usize> = Mutex::new(0);

    macro_rules! setup {
        () => {
            // acquire lock within scope of each test
            let mut guard = TEST_LOCK.lock().unwrap();
            if *guard == 0 {
                crate::init!();
                *guard += 1;
            }
            static mut VEC: Vec<String> = Vec::new();
            let vec_flusher = unsafe { VecFlusher::new(&mut VEC) };
            crate::logger().use_flush(Box::new(vec_flusher));
            crate::logger().use_formatter(Box::new(TestFormatter::new()))
        };
    }

    fn from_log_lines<F: Fn(&str) -> String>(lines: &[String], f: F) -> Vec<String> {
        lines.iter().map(|s| f(s.as_str())).collect::<Vec<_>>()
    }

    #[doc(hidden)]
    macro_rules! helper_assert {
        (@ $f:expr, $format_string:expr, $check_f:expr) => {
            $f;
            flush!();
            assert_eq!(
                unsafe { from_log_lines(&VEC, $check_f) },
                vec![$format_string]
            );
            unsafe {
                let _ = &VEC.clear();
            }
        };
    }

    macro_rules! assert_message_equal {
        ($f:expr, $format_string:expr) => { helper_assert!(@ $f, $format_string, message_from_log_line) };
    }

    macro_rules! assert_message_with_level_equal {
        ($f:expr, $format_string:expr) => { helper_assert!(@ $f, $format_string, message_and_level_from_log_line) };
    }

    #[derive(Clone, Debug)]
    struct NestedSomething {
        thing: Something,
    }

    impl std::fmt::Display for Something {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Something display: {}", self.some_str)
        }
    }

    #[test]
    fn has_all_levels() {
        setup!();

        assert_message_with_level_equal!(
            trace!("Hello world {}", "Another"),
            format!("[TRC]\tHello world {}", "Another")
        );
        assert_message_with_level_equal!(
            debug!("Hello world {}", "Another"),
            format!("[DBG]\tHello world {}", "Another")
        );
        assert_message_with_level_equal!(
            info!("Hello world {}", "Another"),
            format!("[INF]\tHello world {}", "Another")
        );
        assert_message_with_level_equal!(
            warn!("Hello world {}", "Another"),
            format!("[WRN]\tHello world {}", "Another")
        );
        assert_message_with_level_equal!(
            error!("Hello world {}", "Another"),
            format!("[ERR]\tHello world {}", "Another")
        );
    }

    #[test]
    fn works_in_closure() {
        setup!();

        let s1 = Something {
            some_str: "Hello world 1",
        };
        let s2 = Something {
            some_str: "Hello world 2",
        };

        let f = || {
            assert_message_equal!(
                info!("Hello world {} {:?}", s1, s2),
                format!("Hello world {} {:?}", s1, s2)
            );
        };

        f();
    }

    #[test]
    fn works_with_attributes() {
        setup!();

        let s1 = Something {
            some_str: "Hello world 1",
        };
        let s2 = Something {
            some_str: "Hello world 2",
        };
        let nested = NestedSomething {
            thing: Something {
                some_str: "hello nested",
            },
        };

        assert_message_equal!(
            info!("log one attr {}", nested.thing.some_str),
            format!("log one attr {}", nested.thing.some_str)
        );
        assert_message_equal!(
            info!("hello world {} {:?}", s1.some_str, s2.some_str),
            format!("hello world {} {:?}", s1.some_str, s2.some_str)
        );
    }

    #[test]
    fn works_with_box_ref() {
        setup!();

        let s1 = Box::new(Something {
            some_str: "Hello world 1",
        });
        let s2 = Box::new(Something {
            some_str: "Hello world 2",
        });

        assert_message_equal!(
            info!("log single box ref {}", s1.as_ref()),
            format!("log single box ref {}", s1.as_ref())
        );
        assert_message_equal!(
            info!("log multi box ref {} {:?}", s1.as_ref(), s2.as_ref()),
            format!("log multi box ref {} {:?}", s1.as_ref(), s2.as_ref())
        );
    }

    #[test]
    fn works_with_move() {
        setup!();

        let s1 = Something {
            some_str: "Hello world 1",
        };
        let s2 = Something {
            some_str: "Hello world 2",
        };
        let s3 = Something {
            some_str: "Hello world 3",
        };

        assert_message_equal!(
            info!("log multi move {} {:?}", s1, s2),
            format!("log multi move {} {:?}", s1, s2)
        );
        assert_message_equal!(
            info!("log single move {}", s3),
            format!("log single move {}", s3)
        );
    }

    #[test]
    fn works_with_references() {
        setup!();

        let s1 = Something {
            some_str: "Hello world 1",
        };
        let s2 = Something {
            some_str: "Hello world 2",
        };

        assert_message_equal!(
            info!("log single ref: {}", &s1),
            format!("log single ref: {}", &s1)
        );
        assert_message_equal!(
            info!("log multi ref: {} {:?}", &s1, &s2),
            format!("log multi ref: {} {:?}", &s1, &s2)
        );
    }

    fn log_multi_ref_helper(thing: &Something, thing2: &Something) {
        info!("log multi ref {} {:?}", thing, thing2);
    }

    fn log_ref_helper(thing: &Something) {
        info!("log single ref: {}", thing)
    }

    #[test]
    fn works_with_ref_lifetime_inside_fn() {
        setup!();

        let s1 = Something {
            some_str: "Hello world 1",
        };
        let s2 = Something {
            some_str: "Hello world 2",
        };

        assert_message_equal!(log_ref_helper(&s1), format!("log single ref: {}", &s1));
        assert_message_equal!(
            log_multi_ref_helper(&s2, &s1),
            format!("log multi ref {} {:?}", &s2, &s1)
        );
    }

    #[derive(Clone)]
    struct A {
        price: u64,
        symbol: &'static str,
        exch_id: u64,
    }

    impl A {
        fn get_price(&self) -> u64 {
            self.price
        }

        fn get_exch_id(&self) -> u64 {
            self.exch_id
        }

        fn get_symbol(&self) -> &'static str {
            self.symbol
        }
    }

    #[test]
    fn works_with_fn_return_val() {
        setup!();

        let a = A {
            price: 1_521_523,
            symbol: "SomeSymbol",
            exch_id: 642_153_768,
        };

        assert_message_equal!(
            info!(
                "A: price: {} symbol: {} exch_id: {}",
                a.get_price(),
                ?a.get_symbol(),
                %a.get_exch_id()
            ),
            format!(
                "A: price: {} symbol: \"{}\" exch_id: {:?}",
                a.get_price(),
                a.get_symbol(),
                a.get_exch_id()
            )
        );
        assert_message_equal!(
            info!("single call {}", a.get_price()),
            format!("single call {}", a.get_price())
        );
    }

    fn log_ref_and_move(s1: Something, s2r: &Something) {
        info!("Hello world {} {:?}", s1, s2r);
    }

    #[test]
    fn works_with_ref_and_move() {
        setup!();

        let s1 = Something {
            some_str: "Hello world 1",
        };
        let s1_clone = s1.clone();
        let s2 = Something {
            some_str: "Hello world 2",
        };

        assert_message_equal!(
            log_ref_and_move(s1, &s2),
            format!("Hello world {} {:?}", s1_clone, &s2)
        );
        let s3 = Something {
            some_str: "Hello world 3",
        };
        let s4 = Something {
            some_str: "Hello world 4",
        };

        assert_message_equal!(
            info!("ref: {:?}, move: {}", &s2, s3),
            format!("ref: {:?}, move: {}", &s2, s3)
        );
        assert_message_equal!(info!("single ref: {}", &s2), format!("single ref: {}", &s2));
        assert_message_equal!(info!("single move: {}", s4), format!("single move: {}", s4));
    }

    #[test]
    fn works_with_eager_debug_display_hints() {
        setup!();

        let s1 = Something {
            some_str: "Hello world 1",
        };
        let s2 = Something {
            some_str: "Hello world 2",
        };
        let some_str = "hello world";

        assert_message_equal!(
            info!("display {}; eager debug {}; eager display {}, eager display inner field {}", some_str, ?s2, %s1, %s1.some_str),
            format!(
                "display {}; eager debug {:?}; eager display {}, eager display inner field {}",
                some_str, s2, s1, s1.some_str
            )
        );
        assert_message_equal!(
            info!("single eager display: {}", %s2),
            format!("single eager display: {}", s2)
        );
    }

    #[test]
    fn works_with_fields() {
        setup!();

        let s1 = Something {
            some_str: "Hello world 1",
        };
        let s2 = Something {
            some_str: "Hello world 1",
        };
        let s3 = Something {
            some_str: "Hello world 3",
        };
        let s3_clone = s3.clone();

        assert_message_equal!(
            info!("pass by ref {}", some_struct.field1.innerfield.inner = &s1),
            format!("pass by ref some_struct.field1.innerfield.inner={}", &s1)
        );
        assert_message_equal!(
            info!("pass by move {}", some.inner.field = s3),
            format!("pass by move some.inner.field={}", s3_clone)
        );
        assert_message_equal!(
            info!(
                "non-nested field: {}, nested field: {}, pure lit: {}",
                borrow_s2_field = %s2,
                some_inner_field.inner.field.inner.arg = "hello world",
                "pure lit arg" = "another lit arg"
            ),
            format!("non-nested field: borrow_s2_field={}, nested field: some_inner_field.inner.field.inner.arg=hello world, pure lit: pure lit arg=another lit arg", &s2)
        );
        assert_message_equal!(
            info!(
                "pure lit: {}, reuse debug: {}, nested field: {}, able to reuse after pass by ref: {}",
                "pure lit arg" = "another lit arg",
                "able to reuse s1" = ?s1,
                some_inner_field.some.field.included = "hello world",
                able.to.reuse.s2.borrow = &s2
            ),
            format!("pure lit: pure lit arg=another lit arg, reuse debug: able to reuse s1={:?}, nested field: some_inner_field.some.field.included=hello world, able to reuse after pass by ref: able.to.reuse.s2.borrow={}", s1, &s2)
        );
    }

    struct S {
        symbol: String,
    }

    impl Serialize for S {
        fn encode(&self, write_buf: &'static mut [u8]) -> Store {
            fn decode(read_buf: &[u8]) -> String {
                let x = from_utf8(read_buf).unwrap();
                x.to_string()
            }
            write_buf.copy_from_slice(self.symbol.as_bytes());
            Store::new(decode, write_buf)
        }

        fn buffer_size_required(&self) -> usize {
            self.symbol.len()
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct BigStruct {
        vec: [i32; 100],
        some: &'static str,
    }

    impl Serialize for BigStruct {
        fn encode(&self, write_buf: &'static mut [u8]) -> Store {
            fn decode(buf: &[u8]) -> String {
                let (mut _head, mut tail) = buf.split_at(0);
                let mut vec = vec![];
                for _ in 0..100 {
                    (_head, tail) = tail.split_at(4);
                    vec.push(i32::from_le_bytes(_head.try_into().unwrap()));
                }
                let s = from_utf8(tail).unwrap();
                format!("vec: {:?}, str: {}", vec, s)
            }

            let (mut _head, mut tail) = write_buf.split_at_mut(0);
            for i in 0..100 {
                (_head, tail) = tail.split_at_mut(4);
                _head.copy_from_slice(&self.vec[i].to_le_bytes())
            }

            tail.copy_from_slice(self.some.as_bytes());

            Store::new(decode, write_buf)
        }

        fn buffer_size_required(&self) -> usize {
            std::mem::size_of::<i32>() * 100 + self.some.len()
        }
    }

    #[test]
    fn works_with_serialize() {
        setup!();

        let s = S {
            symbol: String::from("Hello"),
        };
        let bs = BigStruct {
            vec: [1; 100],
            some: "The quick brown fox jumps over the lazy dog",
        };

        assert_message_equal!(info!("s: {} {}", ^s, ^s), "s: Hello Hello");
        assert_message_equal!(
            info!("bs: {}", ^bs),
            format!(
                "bs: vec: {:?}, str: {}",
                vec![1; 100],
                "The quick brown fox jumps over the lazy dog"
            )
        );
    }
}
