use quicklog::{fmt::formatter, info, level::Level};

mod common;

#[test]
fn custom_timestamp_format() {
    setup!(
        formatter = formatter()
            .with_ansi(false)
            // Check that provided pattern overrides all other settings
            .with_time_fmt("%F %T%z")
            .with_pattern("%(filename):%(level) {{}} %(message)")
            .build()
    );

    assert_message_equal!(
        info!("Hello world"),
        format!("{}:{} {{}} Hello world", file!(), Level::Info)
    );
}
