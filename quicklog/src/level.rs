//! Defines the levels of verbosity available for logging.
//!
//! ## Logging Levels
//!
//! Defined in [`Level`]. Consists of 6 levels in total:
//!
//! * [`Trace`]
//! * [`Debug`]
//! * [`Info`]
//! * [`Warn`]
//! * [`Error`]
//! * [`Event`]
//!
//! ## LevelFilter
//!
//! [`LevelFilter`] is similar to [`Level`], but is primarily used for
//! determining whether a log gets filtered out based on its `Level`. Hence,
//! `LevelFilter` comes with the addition of [`LevelFilter::Off`]. To modify the
//! current `LevelFilter`, use the [`set_max_level`] function. For instance:
//!
//! ```rust
//! use quicklog::level::{set_max_level, LevelFilter};
//! use quicklog::queue::FlushError;
//! use quicklog::{flush, info, init, error};
//!
//! # fn main() {
//! init!();
//!
//! info!("Info");
//! assert_eq!(flush!(), Ok(()));
//!
//! // only log errors from here on
//! set_max_level(LevelFilter::Error);
//! info!("Info");
//! assert_eq!(flush!(), Err(FlushError::Empty));
//!
//! error!("Error");
//! assert_eq!(flush!(), Ok(()));
//! # }
//! ```
//!
//! In general, logs which tend to have lower *priority* (`Trace`, `Debug`)
//! are considered to have *lower* levels than higher priority levels like
//! `Error`. Logging levels will be skipped if a level is less than the current
//! [`LevelFilter`], i.e. `Level::Debug` is skipped when LevelFilter is set to
//! `Info`.
//!
//! [`Trace`]: crate::level::Level::Trace
//! [`Debug`]: crate::level::Level::Debug
//! [`Info`]: crate::level::Level::Info
//! [`Warn`]: crate::level::Level::Warn
//! [`Error`]: crate::level::Level::Error
//! [`Event`]: crate::level::Level::Event
//! [`Level`]: crate::level::Level
//! [`set_max_level`]: crate::level::set_max_level
//! [`LevelFilter`]: crate::level::LevelFilter
//! [`LevelFilter::Off`]: crate::level::LevelFilter::Off

/// Verbosity of a logging event.
///
/// Note that `Trace` is considered to have the lowest level, and
/// `Error`/`Event` have the highest levels. These levels, along with the current
/// [`LevelFilter`], will determine whether the associated log gets recorded.
/// For instance, if the currently set [`LevelFilter`] has level `Info`, then
/// only logs with levels `Info`, `Warn`, `Error` and `Event` will be recorded.
///
/// # Examples
///
/// ```rust
/// use quicklog::level::{set_max_level, Level, LevelFilter};
/// use quicklog::is_level_enabled;
///
/// # fn main() {
/// assert!(Level::Trace < Level::Debug);
/// assert!(Level::Error > Level::Info);
///
/// // filter comparison -- determines which logs get recorded at runtime
/// set_max_level(LevelFilter::Off);
/// assert!(!is_level_enabled!(Level::Trace));
///
/// set_max_level(LevelFilter::Info);
/// assert!(!is_level_enabled!(Level::Debug));
/// assert!(is_level_enabled!(Level::Info));
/// # }
/// ```
///
/// [`LevelFilter`]: LevelFilter
#[repr(usize)]
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd)]
pub enum Level {
    /// Designates trace information, which is usually of very low priority.
    Trace = 0,
    /// Designates debug information, which is usually of low priority.
    Debug = 1,
    /// Designates useful information.
    Info = 2,
    /// Designates potentially hazardous situations.
    Warn = 3,
    /// Designates serious errors.
    Error = 4,
    /// Designates key-value (e.g JSON) records.
    Event = 5,
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level = match self {
            Self::Trace => "TRC",
            Self::Debug => "DBG",
            Self::Info => "INF",
            Self::Warn => "WRN",
            Self::Error => "ERR",
            Self::Event => "EVT",
        };
        write!(f, "{}", level)
    }
}

impl std::fmt::Debug for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

/// `LevelFilter` represents the different [`Level`]s of logging we have,
/// with the addition of `Off`, which will disable all logging (at runtime).
#[repr(usize)]
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd)]
pub enum LevelFilter {
    /// Enables trace and above.
    Trace = 0,
    /// Enables debug and above.
    Debug = 1,
    /// Enables info and above.
    Info = 2,
    /// Enables warn and above.
    Warn = 3,
    /// Enables Error logs only.
    Error = 4,
    /// Event-level log-records are sets of key-value pairs that are
    /// intended for machine processing.  The formatted log-message
    /// should be a simple record tag, with all the variable data in
    /// key-value pairs.
    Event = 5,
    /// Disables all logging.
    Off = 6,
}

impl std::fmt::Display for LevelFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level_filter = match self {
            Self::Trace => "TRC",
            Self::Debug => "DBG",
            Self::Info => "INF",
            Self::Warn => "WRN",
            Self::Error => "ERR",
            Self::Event => "EVT",
            Self::Off => "OFF",
        };
        write!(f, "{}", level_filter)
    }
}

impl std::fmt::Debug for LevelFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLevelParseError;

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
            _ => Err(LogLevelParseError),
        }
    }
}

#[cfg(debug_assertions)]
static mut MAX_LOG_LEVEL_FILTER: LevelFilter = LevelFilter::Trace;

#[cfg(not(debug_assertions))]
static mut MAX_LOG_LEVEL_FILTER: LevelFilter = LevelFilter::Info;

/// Modifies the maximum log level that will be logged.
///
/// If [`Level`] is greater than or equal to a [`LevelFilter`], then it is
/// enabled. See the documentation for [`Level`] for more details on what this
/// means, as well as the [crate documentation](crate#log-filtering) for an
/// example on how to use this function.
#[inline]
pub fn set_max_level(level: LevelFilter) {
    unsafe {
        MAX_LOG_LEVEL_FILTER = level;
    }
}

/// The currently set [`LevelFilter`]. By default, this is `Trace` in Debug and
/// `Info` in Release.
///
/// Logs with a [`Level`] greater than or equal to the returned [`LevelFilter`]
/// will be enabled, whereas the rest will be disabled.
#[inline(always)]
pub fn max_level() -> LevelFilter {
    unsafe { MAX_LOG_LEVEL_FILTER }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each filter should enable the corresponding level greater than equal to
    /// its index.
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
