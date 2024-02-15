use std::str::FromStr;

use quicklog::{fmt::formatter, info, level::LevelFilter};

mod common;

#[test]
fn no_timestamp() {
    setup!(formatter = formatter().with_ansi(false).without_time().build());

    info!("Hello world");
    flush_all!();
    let level = first_field_from_log_line!();
    LevelFilter::from_str(&level).expect("expected level as first field without timestamp");
}
