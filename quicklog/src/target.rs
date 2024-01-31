use crate::level::LevelFilter;

#[allow(unused)]
struct Filter {
    target: String,
    level: LevelFilter,
}

#[derive(Default)]
pub struct TargetFilter {
    // TODO: consider hashing after profiling performance.
    // For now, since the number of custom target filters is expected to be
    // relatively small, this should be fine.
    filters: Vec<Filter>,
}

impl TargetFilter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a (target, level filter) pair to the set of filters.
    pub fn with_target(mut self, target: impl Into<String>, level: impl Into<LevelFilter>) -> Self {
        self.filters.push(Filter {
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
        self.filters.extend(targets.map(|(target, level)| Filter {
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
