use chrono::DateTime;
use quicklog::{fmt::formatter, info};

mod common;

#[test]
fn custom_timestamp_format() {
    let format = "%F %T%z";
    setup!(formatter = formatter().with_ansi(false).with_time_fmt(format).build());

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
