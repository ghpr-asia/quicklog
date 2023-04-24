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
//! ```norun
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
//! ```norun
//! struct SomeClock;
//!
//! impl Clock for SomeClock { /* impl */ }
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
//! ```norun
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

/// Channel handles the intermediate communication between senders and receivers
/// Each `MacroCallsite` receives its own channel
#[doc(hidden)]
static mut CHANNEL: Lazy<(Sender<Intermediate>, Receiver<Intermediate>)> = Lazy::new(channel);

/// Log is the base trait that Quicklog will implement.
/// Flushing and formatting is deferred while logging.
pub trait Log: Send + Sync {
    fn flush(&self, timeout: Option<Duration>) -> Result<(), RecvTimeoutError>;
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
    pub fn use_flush(&mut self, flush: Container<dyn Flush>) {
        self.flusher = flush
    }

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
    fn flush(&self, maybe_timeout: Option<Duration>) -> Result<(), RecvTimeoutError> {
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
                        self.clock.compute_system_time_from_instant(&time_logged),
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

    fn log(
        &self,
        callsite: &Callsite,
        display: Container<dyn Display>,
    ) -> Result<(), SendError<Intermediate>> {
        match callsite.sender.send((self.clock.get_instant(), display)) {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }
}