//! ## `Flush` trait
//!
//! Simple trait that allows an underlying implementation of Flush to
//! perform some type of IO operation, i.e. writing to file, writing to
//! stdout, etc
//!
//! ## Example usage of `Flush`
//!
//! ```rust
//! use quicklog_flush::Flush;
//! # use quicklog_flush::stdout_flusher::StdoutFlusher;
//! # use std::collections::VecDeque;
//! # fn serialize_into_string(item: String) -> String { item }
//! # struct Quicklog;
//! impl Quicklog {
//!     fn flush_logger(&mut self) {
//!         # let mut flusher = StdoutFlusher::new();
//!         # let mut queue = VecDeque::new();
//!         # queue.push_back(String::from("Hello, world!"));
//!         while let Some(item) = queue.pop_front() {
//!             let log_string = serialize_into_string(item);
//!             // flusher implements `Flush` trait
//!             flusher.flush_one(log_string);
//!         }
//!     }
//! }
//! ```

/// Flushes to a file
pub mod file_flusher;
/// No-op Flush, does nothing
pub mod noop_flusher;
/// Flushes to stderr through `eprint!` macro
pub mod stderr_flusher;
/// Flushes to stdout through `print!` macro
pub mod stdout_flusher;

/// Simple trait that allows an underlying implementation of Flush to
/// perform some type of IO operation, i.e. writing to file, writing to
/// stdout, etc
pub trait Flush {
    /// Handles a string from another thread, and potentially performs I/O
    /// operations such as writing to a file or to stdout
    fn flush_one(&mut self, display: String);
}
