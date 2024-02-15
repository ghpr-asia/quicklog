use quicklog::{fmt::formatter, info};

mod common;

#[test]
fn default_formatter() {
    setup!(formatter = formatter().with_ansi(false).build());

    info!("Hello world");
    flush_all!();
    let ts = first_field_from_log_line!();
    ts.parse::<usize>()
        .expect("cannot parse timestamp; expected unix timestamp");
}
