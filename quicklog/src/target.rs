use std::str::FromStr;

use crate::level::{Level, LevelFilter, DEFAULT_LOG_LEVEL};

#[derive(Debug)]
pub enum FilterParseError {
    MissingTarget(String),
    UnknownLevel(String),
    InvalidFormat(String),
}

impl core::fmt::Display for FilterParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingTarget(s) => {
                f.write_fmt(format_args!("filter {}: no target specified", s))
            }
            Self::UnknownLevel(s) => {
                f.write_fmt(format_args!("filter {}: level not recognized", s))
            }
            Self::InvalidFormat(s) => f.write_fmt(format_args!("filter {}: invalid format", s)),
        }
    }
}

enum FilterTarget {
    Global,
    Module(String),
}

/// Parsed raw target filter.
#[allow(unused)]
struct RawFilter {
    target: FilterTarget,
    level: LevelFilter,
}

impl FromStr for RawFilter {
    type Err = FilterParseError;

    /// Heavily adapted from `env_logger`:
    /// https://github.com/rust-cli/env_logger/blob/9303b0c0393c33046a791b0a6497b0f03ef1f434/crates/env_filter/src/parser.rs#L8.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split('=');

        let (target, level) = match (split.next(), split.next().map(|s| s.trim()), split.next()) {
            (Some(possible_target), None, None) => {
                // Check if the prefix is parseable as a Level. If so, treat this
                // as the new global level
                possible_target
                    .parse::<LevelFilter>()
                    .map(|filter| (FilterTarget::Global, filter))
                    .unwrap_or_else(|_| {
                        (
                            FilterTarget::Module(possible_target.to_string()),
                            LevelFilter::Trace,
                        )
                    })
            }
            (Some(possible_target), Some(""), None) => (
                FilterTarget::Module(possible_target.to_string()),
                LevelFilter::Trace,
            ),
            (Some(possible_target), Some(s), None) => (
                FilterTarget::Module(possible_target.to_string()),
                s.parse::<LevelFilter>()
                    .map_err(|_| FilterParseError::UnknownLevel(s.to_string()))?,
            ),
            _ => return Err(FilterParseError::InvalidFormat(s.to_string())),
        };

        Ok(Self { target, level })
    }
}

/// Final form of a valid target filter.
///
/// Follows syntax of the form `target=level`.
#[allow(unused)]
#[derive(Debug, Clone)]
pub struct TargetFilter {
    target: String,
    level: LevelFilter,
}

/// Collection of target filters.
#[derive(Debug, Default)]
pub struct TargetFilters {
    // TODO: consider hashing after profiling performance
    // For now, since the number of custom target filters is expected to be
    // relatively small, this should be fine.
    filters: Vec<TargetFilter>,
}

impl TargetFilters {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a (target, level filter) pair to the set of filters.
    pub fn with_target(mut self, target: impl Into<String>, level: impl Into<LevelFilter>) -> Self {
        self.filters.push(TargetFilter {
            target: target.into(),
            level: level.into(),
        });

        self
    }

    /// Adds multiple (target, level filter) pairs to the set of filters.
    pub fn with_targets<T, L>(mut self, targets: impl Iterator<Item = (T, L)>) -> Self
    where
        T: Into<String>,
        L: Into<LevelFilter>,
    {
        self.filters
            .extend(targets.map(|(target, level)| TargetFilter {
                target: target.into(),
                level: level.into(),
            }));

        self
    }

    #[cfg(feature = "target-filter")]
    pub(crate) fn target_level(&self, target: &str) -> Option<LevelFilter> {
        self.filters
            .iter()
            .find_map(|filter| (filter.target.as_str() == target).then_some(filter.level))
    }
}

/// Resolver for global and specific target filters.
pub(crate) struct Filter {
    pub(crate) global: LevelFilter,
    #[cfg(feature = "target-filter")]
    pub(crate) target_filters: Option<TargetFilters>,
}

impl Filter {
    /// Logs with a [`Level`] greater than or equal to the returned [`LevelFilter`]
    /// will be enabled, whereas the rest will be disabled.
    #[inline(always)]
    pub(crate) fn is_level_enabled(&self, level: Level) -> bool {
        self.global.is_enabled(level)
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
                .target_filters
                .as_ref()
                .and_then(|filter| filter.target_level(_target))
            else {
                return self.is_level_enabled(level);
            };

            target_level.is_enabled(level)
        }
    }

    /// Checks the current set of [`TargetFilters`] against incoming ones.
    ///
    /// If there is a target conflict, then the stricter [`LevelFilter`] will
    /// override the existing one.
    /// Otherwise, the filter is just added to the current set.
    #[cfg(feature = "target-filter")]
    pub(crate) fn resolve_filters(mut self, mut target_filters: TargetFilters) -> Self {
        let Some(current_filters) = self.target_filters.take() else {
            return Self {
                global: self.global,
                target_filters: (!target_filters.filters.is_empty()).then_some(target_filters),
            };
        };

        // Take the stricter of both sets of filters for every matching target
        let mut new_filters =
            Vec::with_capacity(current_filters.filters.len() + target_filters.filters.len());
        for filter in current_filters.filters {
            if let Some(competing_filter_idx) = target_filters
                .filters
                .iter()
                .position(|f| f.target == filter.target)
            {
                let competing_filter = target_filters.filters.swap_remove(competing_filter_idx);

                if competing_filter.level as usize >= filter.level as usize {
                    new_filters.push(competing_filter);
                } else {
                    new_filters.push(filter)
                }

                continue;
            }

            new_filters.push(filter);
        }
        // Add remaining target filters without a match
        new_filters.append(&mut target_filters.filters);

        Self {
            global: self.global,
            target_filters: Some(TargetFilters {
                filters: new_filters,
            }),
        }
    }

    #[cfg(feature = "target-filter")]
    fn parse_str(s: &str) -> Self {
        let mut filters = TargetFilters::default();
        let mut global_log_level = DEFAULT_LOG_LEVEL;

        for raw_filter_res in s.split(',').map(RawFilter::from_str) {
            match raw_filter_res {
                Ok(RawFilter {
                    target: FilterTarget::Global,
                    level,
                }) => {
                    global_log_level = level;
                }
                Ok(RawFilter {
                    target: FilterTarget::Module(s),
                    level,
                }) => {
                    filters = filters.with_target(s, level);
                }
                Err(e) => {
                    eprintln!("Error in parsing RUST_LOG: {}", e);
                }
            }
        }

        Self {
            global: global_log_level,
            target_filters: (!filters.filters.is_empty()).then_some(filters),
        }
    }
}

#[cfg(not(feature = "target-filter"))]
impl Default for Filter {
    fn default() -> Self {
        Self {
            global: DEFAULT_LOG_LEVEL,
        }
    }
}

#[cfg(feature = "target-filter")]
impl Default for Filter {
    fn default() -> Self {
        std::env::var("RUST_LOG")
            .ok()
            .map(|s| Filter::parse_str(&s))
            .unwrap_or_else(|| Filter {
                global: DEFAULT_LOG_LEVEL,
                target_filters: None,
            })
    }
}

#[cfg(feature = "target-filter")]
#[cfg(test)]
mod tests {
    use super::Filter;
    use crate::level::LevelFilter;
    use crate::level::DEFAULT_LOG_LEVEL;

    #[test]
    fn valid_filter() {
        let filter = Filter::parse_str("crate1::module_1=info,crate2::module2::n_module3=warn");
        assert_eq!(filter.global, DEFAULT_LOG_LEVEL);

        let target_filters = filter.target_filters.as_ref().unwrap();
        assert!(target_filters.filters.len() == 2);
        assert_eq!(
            target_filters.filters.get(0).unwrap().target,
            "crate1::module_1"
        );
        assert_eq!(
            target_filters.filters.get(0).unwrap().level,
            LevelFilter::Info
        );

        assert_eq!(
            target_filters.filters.get(1).unwrap().target,
            "crate2::module2::n_module3"
        );
        assert_eq!(
            target_filters.filters.get(1).unwrap().level,
            LevelFilter::Warn
        );
    }

    #[test]
    fn valid_filter_case_insensitive() {
        let filter = Filter::parse_str("crate1=iNfO");
        assert_eq!(filter.global, DEFAULT_LOG_LEVEL);

        let target_filters = filter.target_filters.as_ref().unwrap();
        assert!(target_filters.filters.len() == 1);
        assert_eq!(target_filters.filters.get(0).unwrap().target, "crate1");
        assert_eq!(
            target_filters.filters.get(0).unwrap().level,
            LevelFilter::Info
        );
    }

    #[test]
    fn valid_filter_global() {
        let filter = Filter::parse_str("off");
        assert_eq!(filter.global, LevelFilter::Off);

        // Last global filter specified is taken
        let filter = Filter::parse_str("info,warn,error");
        assert_eq!(filter.global, LevelFilter::Error);

        // Globally off, but override for some modules
        let filter = Filter::parse_str("off,crate1::module_1=warn");
        assert_eq!(filter.global, LevelFilter::Off);

        let target_filters = filter.target_filters.as_ref().unwrap();
        assert!(target_filters.filters.len() == 1);
        assert_eq!(
            target_filters.filters.get(0).unwrap().target,
            "crate1::module_1"
        );
        assert_eq!(
            target_filters.filters.get(0).unwrap().level,
            LevelFilter::Warn
        );
    }

    #[test]
    fn invalid_level() {
        let filter = Filter::parse_str("crate1=unknown,crate2=info");
        assert_eq!(filter.global, DEFAULT_LOG_LEVEL);

        let target_filters = filter.target_filters.as_ref().unwrap();
        assert!(target_filters.filters.len() == 1);
        assert_eq!(target_filters.filters.get(0).unwrap().target, "crate2");
        assert_eq!(
            target_filters.filters.get(0).unwrap().level,
            LevelFilter::Info
        );
    }

    #[test]
    fn empty_level() {
        // Empty level defaults to all logging enabled
        let filter = Filter::parse_str("crate1=unknown,crate2=");

        let target_filters = filter.target_filters.as_ref().unwrap();
        assert!(target_filters.filters.len() == 1);
        assert_eq!(target_filters.filters.get(0).unwrap().target, "crate2");
        assert_eq!(
            target_filters.filters.get(0).unwrap().level,
            LevelFilter::Trace
        );
    }

    #[test]
    fn invalid_format() {
        let filter = Filter::parse_str("crate1=info=warn,crate2=error");
        assert_eq!(filter.global, DEFAULT_LOG_LEVEL);

        let target_filters = filter.target_filters.as_ref().unwrap();
        assert!(target_filters.filters.len() == 1);
        assert_eq!(target_filters.filters.get(0).unwrap().target, "crate2");
        assert_eq!(
            target_filters.filters.get(0).unwrap().level,
            LevelFilter::Error
        );
    }
}
