//! An asynchronous logger where formatting and I/O is deferred.
//!
//! # Overview
//!
//! `Quicklog` is provides a framework for logging where it allows for deferred
//! deferred formatting and deferred I/O of logging, which should in turn provide
//! more performant logging with low callsite latency.
//!
//! ## Deferred Formatting
//!
//! ### Why?
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
//! ### Why?
//!
//! Deferring the I/O of formatting would allow for low callsite latency and allow
//! a user to implement their own flush site, possibly on a separate thread
//!
//! # Usage
//!
//! ## Example Usage
//!
//! ```ignore
//! use std::thread;
//!
//! use quicklog::{info, flush};
//!
//! fn main() {
//!     info!("hello world! {}", "some argument");
//!     thread::spawn(|| {
//!         flush!();
//!     });
//! }
//! ```
//!
//! # Macros
//!
//! ## Shorthand Macros
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
//! ## Setup Macros
//!
//! Quicklog allows a user specified [`Clock`] or [`Flush`] to be implemented by
//! the user. This can be passed in through these macros, as long as the
//! underlying struct implements the correct traits
//!
//! * [`with_clock!`]
//! * [`with_flush!`]
//!
//! ## Macro prefix for eager evaluation
//!
//! There are two prefixes you can use for variables, `%` and `?`. This works the same
//! way as `tracing`, where `%` eagerly evaluates an object that implements `Display`
//! and `?` eagerly evaluates an object that implements `Debug`.
//!
//! ```ignore
//! fn main {
//!     info!("eager display {}; eager debug {}", %display_struct, ?debug_struct);
//! }
//! ```
//!
//! ## Structured fields
//!
//! Structured fields in log lines can be specified using `field_name = field_value`
//! syntax. `field_name` can be a literal or a bunch of idents. This can also
//! be used in combination with '%' and '?' prefix on args to eagerly evaluate
//! expressions into format strings.
//!
//! ```ignore
//! fn main {
//!     let value = 10;
//!     info!("hello world; {}; {}", "some string field" = true, "value is" = %value);
//!     // output: "hello world; some string field=true; value is=10"
//!     info!("hello world {} {} {}", question.tricky = true, question.answer = ?value, question.val = &value);
//!     // output: "hello world question.tricky=true question.answer=10 question.val=10"
//! }
//! ```
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
//!     with_clock!(SomeClock::new());
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
//! ```ignore
//! use quicklog_flush::file_flusher::FileFlusher;
//!
//! fn main() {
//!     with_flush!(FileFlusher::new("some/new/location/logs.log"));
//!     // uses the new FileFlusher passed in for flushing
//!     flush!();
//! }
//! ```

use callsite::Callsite;
use once_cell::sync::Lazy;
use quanta::Instant;
use std::{
    fmt::Display,
    sync::mpsc::{channel, Receiver, RecvTimeoutError, SendError, Sender},
    time::Duration,
};

use quicklog_clock::{quanta::QuantaClock, Clock};
use quicklog_flush::{file_flusher::FileFlusher, Flush};

pub mod callsite;
pub mod level;
pub mod macros;

pub type Container<T> = Box<T>;

pub type Intermediate = (Instant, Container<dyn Display>);

/// Logger initialized to Quicklog
#[doc(hidden)]
static mut LOGGER: Lazy<Quicklog> = Lazy::new(Quicklog::default);

pub type RecvResult = Result<(), RecvTimeoutError>;
pub type SendResult = Result<(), SendError<Intermediate>>;
/// Channel handles the intermediate communication between senders and receivers
/// Each `MacroCallsite` receives its own channel
#[doc(hidden)]
static mut CHANNEL: Lazy<(Sender<Intermediate>, Receiver<Intermediate>)> = Lazy::new(channel);

/// Log is the base trait that Quicklog will implement.
/// Flushing and formatting is deferred while logging.
pub trait Log: Send + Sync {
    fn flush(&mut self, timeout: Option<Duration>) -> Result<(), RecvTimeoutError>;
    fn log(
        &self,
        callsite: &Callsite,
        display: Container<dyn Display>,
    ) -> Result<(), SendError<Intermediate>>;
}

/// Returns a mut reference to the globally static logger [`LOGGER`]
///
/// Used internally for macros
#[doc(hidden)]
pub fn logger() -> &'static mut Quicklog {
    unsafe { &mut LOGGER }
}

/// Makes a clone of the sender channel from static [`CHANNEL`]
#[doc(hidden)]
pub fn clone_sender() -> Sender<Intermediate> {
    unsafe { CHANNEL.0.clone() }
}

/// Quicklog implements the Log trait, as well as providing allow
/// a `MacroCallsite` in order to get their own senders, which would then be used
/// to send data onto the single receiver which is flushed when `flush()` is
/// called on the `Log` trait
pub struct Quicklog {
    flusher: Container<dyn Flush>,
    clock: Container<dyn Clock>,
}

impl Quicklog {
    #[doc(hidden)]
    pub fn use_flush(&mut self, flush: Container<dyn Flush>) {
        self.flusher = flush
    }

    #[doc(hidden)]
    pub fn use_clock(&mut self, clock: Container<dyn Clock>) {
        self.clock = clock
    }
}

impl Default for Quicklog {
    fn default() -> Self {
        Quicklog {
            flusher: Container::new(FileFlusher::new("logs/quicklog.log")),
            clock: Container::new(QuantaClock::new()),
        }
    }
}

/// Sync is implemented for Quicklog as the underlying queue used is `mpsc` and
/// the queue would be thread safe on sharing
unsafe impl Send for Quicklog {}
unsafe impl Sync for Quicklog {}

impl Log for Quicklog {
    fn flush(&mut self, maybe_timeout: Option<Duration>) -> RecvResult {
        /// Defines timeout duration we wait before we timeout when flushing from
        /// the channel's receiver
        const DEFAULT_TIMEOUT_DURATION: Duration = Duration::from_micros(1);
        let timeout = maybe_timeout.unwrap_or(DEFAULT_TIMEOUT_DURATION);

        // Flushes until we reach an error, timeout is not considered an error
        // Since we simply reached the end of all the messages we wanted to log
        loop {
            match unsafe { CHANNEL.1.recv_timeout(timeout) } {
                Ok((time_logged, disp)) => {
                    let log_line = format!(
                        "[{:?}]{}\n",
                        self.clock
                            .compute_system_time_from_instant(time_logged)
                            .expect("Unable to get time from instant"),
                        disp
                    );
                    self.flusher.flush(log_line);
                }
                Err(err) => match err {
                    RecvTimeoutError::Timeout => return Ok(()),
                    RecvTimeoutError::Disconnected => return Err(RecvTimeoutError::Disconnected),
                },
            }
        }
    }

    fn log(&self, callsite: &Callsite, display: Container<dyn Display>) -> SendResult {
        match callsite.sender.send((self.clock.get_instant(), display)) {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use quicklog_flush::Flush;

    use crate::{debug, error, flush, info, trace, warn};

    struct VecFlusher {
        pub vec: &'static mut Vec<String>,
    }

    impl VecFlusher {
        pub fn new(vec: &'static mut Vec<String>) -> VecFlusher {
            VecFlusher { vec }
        }
    }

    impl Flush for VecFlusher {
        fn flush(&mut self, display: String) {
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

    // tests need to be single threaded, this mutex ensures
    // tests are only executed in single threaded mode
    static TEST_LOCK: Mutex<usize> = Mutex::new(0);

    macro_rules! setup {
        () => {
            // acquire lock within scope of each test
            let _guard = TEST_LOCK.lock().unwrap();
            static mut VEC: Vec<String> = Vec::new();
            let vec_flusher = unsafe { VecFlusher::new(&mut VEC) };
            crate::logger().use_flush(Box::new(vec_flusher));
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
            format!("[TRACE]\tHello world {}", "Another")
        );
        assert_message_with_level_equal!(
            debug!("Hello world {}", "Another"),
            format!("[DEBUG]\tHello world {}", "Another")
        );
        assert_message_with_level_equal!(
            info!("Hello world {}", "Another"),
            format!("[INFO]\tHello world {}", "Another")
        );
        assert_message_with_level_equal!(
            warn!("Hello world {}", "Another"),
            format!("[WARN]\tHello world {}", "Another")
        );
        assert_message_with_level_equal!(
            error!("Hello world {}", "Another"),
            format!("[ERROR]\tHello world {}", "Another")
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
}
