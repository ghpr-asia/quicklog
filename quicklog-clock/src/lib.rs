//! `Clock` dictates how timestamps are done in the Quicklog.
//! The idea is to use TSC time, storing a start TSC time, and a start System time.
//!
//! We store TSC time when logging is done in Quicklog, only getting the System
//! time on the writer thread that is performance sensitive, as we would
//! be able to decode the true System time, given the delta between
//! `Instant` of generating the log line and `Instant` of the start time to get a
//! `Duration`, which can be added to start System time to give a final `DateTime<Utc>`.
//!
//! Here's an example of how things are done in time taking.
//!
//! ```rust no_run
//! use std::thread;
//! use quicklog_clock::{Clock, quanta::QuantaClock};
//!
//! // initialize the clock, impls `Clock` trait
//! let clock = QuantaClock::new();
//!
//! let some_log_line_instant = clock.get_instant();
//! // add log_line onto some queue
//!
//! // simulate flush thread
//! let flush_thread = thread::spawn(move || {
//!     // some code to flush log lines
//!     let actual_system_time = clock.compute_system_time_from_instant(some_log_line_instant);
//! });
//!
//! # flush_thread.join();
//! ```

use ::quanta::Instant;
use chrono::{DateTime, OutOfRangeError, Utc};

pub mod quanta;

pub trait Clock {
    /// Returns current tsc instant
    fn get_instant(&self) -> Instant;
    /// Returns system time from TSC time
    fn compute_system_time_from_instant(
        &self,
        instant: Instant,
    ) -> Result<DateTime<Utc>, OutOfRangeError>;
}
