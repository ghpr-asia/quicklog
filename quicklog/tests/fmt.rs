use std::str::FromStr;

use chrono::{DateTime, Local};
use quicklog::{fmt::formatter, info, level::LevelFilter};

mod common;

#[test]
fn formatter_config() {
    setup!();

    // Default (without ansi)
    {
        formatter().with_ansi(false).init();

        info!("Hello world");
        flush_all!();
        let ts = first_field_from_log_line!();
        ts.parse::<usize>()
            .expect("cannot parse timestamp; expected unix timestamp");
    }

    // No timestamp
    {
        formatter().with_ansi(false).without_time().init();

        info!("Hello world");
        flush_all!();
        let level = first_field_from_log_line!();
        LevelFilter::from_str(&level).expect("expected level as first field without timestamp");
    }

    // Custom timestamp format
    {
        let format = "%F %T%z";
        formatter().with_ansi(false).with_time_fmt(format).init();

        info!("Hello world");
        flush_all!();
        let ts = first_field_from_log_line!();

        // able to parse with format
        let dt = DateTime::parse_from_str(&ts, format).unwrap_or_else(|_| {
            panic!(
                "failed to parse timestamp str with specified format: {}",
                format,
            )
        });

        // default UTC
        assert_eq!(dt.offset().local_minus_utc(), 0);
    }

    // Custom timestamp format with local timezone
    {
        let format = "%F %T%z";
        formatter()
            .with_ansi(false)
            .with_time_local()
            .with_time_fmt(format)
            .init();

        info!("Hello world");
        flush_all!();
        let ts = first_field_from_log_line!();

        // able to parse with format
        let dt = DateTime::parse_from_str(&ts, format).unwrap_or_else(|_| {
            panic!(
                "failed to parse timestamp str with specified format: {}",
                format,
            )
        });

        // local timezone
        let local_offset_dur = Local::now().offset().local_minus_utc();
        assert_eq!(dt.offset().local_minus_utc(), local_offset_dur);
    }
}
