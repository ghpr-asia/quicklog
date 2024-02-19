use quicklog::{fmt::formatter, info};

mod common;

#[test]
fn custom_timestamp_format() {
    setup!(
        formatter = formatter()
            .with_ansi(false)
            .with_level(false)
            .without_time()
            // Invalid due to missing delim -- should not be used
            .with_pattern("Hello world %(time) %(level) %(message")
            .build()
    );

    assert_message_equal!(info!("Hello world"), "Hello world");
}
