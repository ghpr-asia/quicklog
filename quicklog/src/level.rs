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
//! use quicklog::level::LevelFilter;
//! use quicklog::FlushError;
//! use quicklog::{error, flush, info, init, set_max_level};
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
//! [`LevelFilter`]: crate::level::LevelFilter
//! [`LevelFilter::Off`]: crate::level::LevelFilter::Off
//! [`set_max_level`]: crate::set_max_level

#[cfg(feature = "ansi")]
use nu_ansi_term::{Color, Style};

#[cfg(debug_assertions)]
pub(crate) const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::Trace;

#[cfg(not(debug_assertions))]
pub(crate) const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::Info;

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
/// use quicklog::level::{Level, LevelFilter};
/// use quicklog::{
///     config, debug, flush, info, init, set_max_level, trace, FlushError, NoopFlusher,
/// };
///
/// # fn main() {
/// init!(config().flusher(NoopFlusher));
/// assert!(Level::Trace < Level::Debug);
/// assert!(Level::Error > Level::Info);
///
/// // filter comparison -- determines which logs get recorded at runtime
/// set_max_level(LevelFilter::Off);
/// trace!("This should not be visible");
/// assert_eq!(flush!(), Err(FlushError::Empty));
///
/// set_max_level(LevelFilter::Info);
/// debug!("This should not be visible");
/// assert_eq!(flush!(), Err(FlushError::Empty));
///
/// info!("This should be visible");
/// assert_eq!(flush!(), Ok(()));
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

impl Level {
    fn name(&self) -> &'static str {
        match self {
            Self::Trace => "TRC",
            Self::Debug => "DBG",
            Self::Info => "INF",
            Self::Warn => "WRN",
            Self::Error => "ERR",
            Self::Event => "EVT",
        }
    }
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl std::fmt::Debug for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

/// `LevelFilter` represents the different [`Level`]s of logging we have,
/// with the addition of `Off`, which will disable all logging (at runtime).
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd)]
pub(crate) struct LevelFormat {
    level: Level,
    #[cfg(feature = "ansi")]
    ansi: bool,
}

impl LevelFormat {
    #[cfg(feature = "ansi")]
    pub(crate) fn new(level: Level, ansi: bool) -> Self {
        Self { level, ansi }
    }

    #[cfg(not(feature = "ansi"))]
    pub(crate) fn new(level: Level) -> Self {
        Self { level }
    }
}

impl std::fmt::Display for LevelFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(feature = "ansi")]
        {
            if self.ansi {
                let name = self.level.name();
                let color = match self.level {
                    Level::Trace => Color::Purple,
                    Level::Debug => Color::Blue,
                    Level::Info => Color::Green,
                    Level::Warn => Color::Yellow,
                    Level::Error => Color::Red,
                    Level::Event => Color::Magenta,
                };
                let style = Style::new().bold().fg(color);

                return write!(f, "{}", style.paint(name));
            }
        }

        write!(f, "{}", self.level)
    }
}

/// `LevelFilter` represents the different [`Level`] of logging we have,
/// with the addition of `Off`.
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

impl LevelFilter {
    #[inline(always)]
    pub(crate) fn is_enabled(&self, level: Level) -> bool {
        level as usize >= *self as usize
    }
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
        match s.to_lowercase().as_str() {
            "trc" | "trace" | "0" => Ok(Self::Trace),
            "dbg" | "debug" | "1" => Ok(Self::Debug),
            "inf" | "info" | "2" => Ok(Self::Info),
            "wrn" | "warn" | "3" => Ok(Self::Warn),
            "err" | "error" | "4" => Ok(Self::Error),
            "evt" | "event" | "5" => Ok(Self::Event),
            "off" => Ok(Self::Off),
            _ => Err(LogLevelParseError),
        }
    }
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
