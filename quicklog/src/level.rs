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
//! [`Trace`]: crate::level::Level::Trace
//! [`Debug`]: crate::level::Level::Debug
//! [`Info`]: crate::level::Level::Info
//! [`Warn`]: crate::level::Level::Warn
//! [`Error`]: crate::level::Level::Error
//! [`Level`]: crate::level::Level
//! [`LevelFilter`]: crate::level::LevelFilter

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

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: Static comparison between enum variants and number of level strings present
        const LEVEL_STRINGS: [&str; 6] = ["TRC", "DBG", "INF", "WRN", "ERR", "OFF"];
        write!(f, "{}", LEVEL_STRINGS[*self as usize])
    }
}

impl std::fmt::Debug for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
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
    /// Event-level log-records are sets of key-value pairs that are
    /// intended for machine processing.  The formatted log-message
    /// should be a simple record tag, with all the variable data in
    /// key-value pairs.
    Event = 5,
    /// Disables all logging
    Off = 6,
}

impl std::fmt::Display for LevelFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: Static comparison between enum variants and number of level filter strings present
        const LEVEL_FILTER_STRINGS: [&str; 6] = ["TRC", "DBG", "INF", "WRN", "ERR", "OFF"];
        write!(f, "{}", LEVEL_FILTER_STRINGS[*self as usize])
    }
}

impl std::fmt::Debug for LevelFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLevelParseError();

impl std::str::FromStr for LevelFilter {
    type Err = LogLevelParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "TRC" => Ok(Self::Trace),
            "DBG" => Ok(Self::Debug),
            "INF" => Ok(Self::Info),
            "WRN" => Ok(Self::Warn),
            "ERR" => Ok(Self::Error),
            "OFF" => Ok(Self::Off),
            "EVT" => Ok(Self::Event),
            _ => Err(LogLevelParseError()),
        }
    }
}

static mut MAX_LOG_LEVEL_FILTER: LevelFilter = LevelFilter::Trace;

#[inline]
pub fn set_max_level(level: LevelFilter) {
    unsafe {
        MAX_LOG_LEVEL_FILTER = level;
    }
}

#[inline(always)]
pub fn max_level() -> LevelFilter {
    unsafe { MAX_LOG_LEVEL_FILTER }
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
