use chrono::{DateTime, Duration, OutOfRangeError, Utc};
use quanta::Instant;

use crate::Clock;

pub struct QuantaClock {
    clock: quanta::Clock,
    start_time: DateTime<Utc>,
    start_instant: quanta::Instant,
}

impl QuantaClock {
    pub fn new() -> QuantaClock {
        let clock = quanta::Clock::new();
        // this also lazily initializes a global clock which
        // can take up to 200ms if it is not initialized
        let start_instant = clock.now();
        QuantaClock {
            clock,
            start_time: Utc::now(),
            start_instant,
        }
    }
}

impl Default for QuantaClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for QuantaClock {
    fn get_instant(&self) -> Instant {
        self.clock.now()
    }

    fn compute_system_time_from_instant(
        &self,
        instant: Instant,
    ) -> Result<DateTime<Utc>, OutOfRangeError> {
        let elapsed_time = instant.duration_since(self.start_instant);
        let chrono_duration = Duration::from_std(elapsed_time);
        chrono_duration.map(|duration| self.start_time + duration)
    }
}
