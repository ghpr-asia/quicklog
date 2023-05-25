//! Defines the levels of verbosity available for logging, as well as LevelFilter.
//!
//! ## Logging Levels
//!
//! Defined in [`Level`]. Consists of 5 levels in total:
//!
//! * [`Trace`]
//! * [`Debug`]
//! * [`Info`]
//! * [`Warn`]
//! * [`Error`]
//!
//! ## LevelFilters
//!
//! Similarly, [`LevelFilter`] correspond to [`Level'], with the addition
//! of [`LevelFilter::Off`].
//!
//! Logging levels will be skipped if a level is lower than current [`LevelFilter`],
//! i.e. `Level::Debug` is skipped when LevelFilter is set to `Info`, but `Level::Info`
//! logs will not be skipped.
//!
//! ### Compile time filter specification
//!
//! Level filter can be set at runtime, via Cargo features. This level is configured
//! separately for release and debug builds through the following feature flags:
//!
//! * `max_level_off`
//! * `max_level_error`
//! * `max_level_warn`
//! * `max_level_info`
//! * `max_level_debug`
//! * `max_level_trace`
//! * `release_max_level_off`
//! * `release_max_level_error`
//! * `release_max_level_warn`
//! * `release_max_level_info`
//! * `release_max_level_debug`
//! * `release_max_level_trace`
//!
//! These features control the value of the const [`MAX_LOG_LEVEL`] constant. The
//! log macros check this value before logging. By default, no levels are disabled.
//!
//! For example, a crate can disable trace level logs in debug builds
//! and trace, debug, and info level logs in release builds with the
//! following configuration:
//!
//! ```toml
//! [dependencies]
//! quicklog = { version = "0.1", features = ["max_level_debug", "release_max_level_warn"] }
//! ```
//!
//! [`Trace`]: crate::level::Level::Trace
//! [`Debug`]: crate::level::Level::Debug
//! [`Info`]: crate::level::Level::Info
//! [`Warn`]: crate::level::Level::Warn
//! [`Error`]: crate::level::Level::Error
//! [`Level`]: crate::level::Level
//! [`LevelFilter`]: crate::level::LevelFilter
//! [`MAX_LOG_LEVEL`]: crate::level::MAX_LOG_LEVEL

use std::fmt::Display;

#[repr(usize)]
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd)]
pub enum Level {
    /// Designates trace information, which is of very low priority
    Trace = 0,
    /// Designates debug information, which is of low priority
    Debug = 1,
    /// Designates useful information
    Info = 2,
    /// Designates potentially hazardous situations
    Warn = 3,
    /// Designates serious errors
    Error = 4,
}

impl Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: Static comparison between enum variants and number of level strings present
        const LEVEL_STRINGS: [&str; 5] = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];
        write!(f, "{}", LEVEL_STRINGS[*self as usize])
    }
}

/// `LevelFilter` represents the different [`Level`] of logging we have,
/// with the addition of `Off`.
#[repr(usize)]
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd)]
#[doc(hidden)]
pub enum LevelFilter {
    /// Enables trace and above
    Trace = 0,
    /// Enables debug and above
    Debug = 1,
    /// Enables info and above
    Info = 2,
    /// Enables warn and above
    Warn = 3,
    /// Enables Error logs only
    Error = 4,
    /// Disables all logging
    Off = 5,
}

impl Display for LevelFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: Static comparison between enum variants and number of level filter strings present
        const LEVEL_FILTER_STRINGS: [&str; 6] = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR", "OFF"];
        write!(f, "{}", LEVEL_FILTER_STRINGS[*self as usize])
    }
}

/// Statically configured maximum level, this is configured from Cargo.toml,
/// by passing in the relevant feature flags
/// By default, the level would be configured to LevelFilter::Trace
pub const MAX_LOG_LEVEL: LevelFilter = MAX_LEVEL;

// Checks the feature flag specified by the user of the library, and sets the
// const [`MAX_LEVEL`] accordingly. Defaults to `LevelFilter::Trace` if the
// user does not specify any feature flag
cfg_if::cfg_if! {
    if #[cfg(all(not(debug_assertions), feature = "release_max_level_off"))] {
        const MAX_LEVEL: LevelFilter = LevelFilter::Off;
    } else if #[cfg(all(not(debug_assertions), feature = "release_max_level_error"))] {
        const MAX_LEVEL: LevelFilter = LevelFilter::Error;
    } else if #[cfg(all(not(debug_assertions), feature = "release_max_level_warn"))] {
        const MAX_LEVEL: LevelFilter = LevelFilter::Warn;
    } else if #[cfg(all(not(debug_assertions), feature = "release_max_level_info"))] {
        const MAX_LEVEL: LevelFilter = LevelFilter::Info;
    } else if #[cfg(all(not(debug_assertions), feature = "release_max_level_debug"))] {
        const MAX_LEVEL: LevelFilter = LevelFilter::Debug;
    } else if #[cfg(all(not(debug_assertions), feature = "release_max_level_trace"))] {
        const MAX_LEVEL: LevelFilter = LevelFilter::Trace;
    } else if #[cfg(feature = "max_level_off")] {
        const MAX_LEVEL: LevelFilter = LevelFilter::Off;
    } else if #[cfg(feature = "max_level_error")] {
        const MAX_LEVEL: LevelFilter = LevelFilter::Error;
    } else if #[cfg(feature = "max_level_warn")] {
        const MAX_LEVEL: LevelFilter = LevelFilter::Warn;
    } else if #[cfg(feature = "max_level_info")] {
        const MAX_LEVEL: LevelFilter = LevelFilter::Info;
    } else if #[cfg(feature = "max_level_debug")] {
        const MAX_LEVEL: LevelFilter = LevelFilter::Debug;
    } else {
        const MAX_LEVEL: LevelFilter = LevelFilter::Trace;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each filter should enable the cooresponding level
    /// greater than equal to its index.
    ///
    /// `LevelFilter::Off` should disable all log levels.
    ///
    /// ### Example
    ///
    /// `LevelFilter::Info` enables:
    /// - `Level::Info`
    /// - `Level::Warn`
    /// - `Level::Error`
    #[test]
    fn compare_level_and_filters() {
        // level should be in increasing order
        let levels = [
            Level::Trace,
            Level::Debug,
            Level::Info,
            Level::Warn,
            Level::Error,
        ];
        let filters = [
            LevelFilter::Trace,
            LevelFilter::Debug,
            LevelFilter::Info,
            LevelFilter::Warn,
            LevelFilter::Error,
            LevelFilter::Off,
        ];
        for (filter_idx, &filter) in filters.iter().enumerate() {
            for (level_idx, &level) in levels.iter().enumerate() {
                let level_val = level as usize;
                let filter_val = filter as usize;
                if level_idx < filter_idx {
                    assert!(level_val < filter_val);
                } else {
                    assert!(level_val >= filter_val);
                }
            }
        }
    }
}
