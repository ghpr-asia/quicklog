use quicklog::{
    error, event, flush, formatter::QuickLogFormatter, info, init, with_formatter,
    with_json_formatter,
};

// Demonstrates how to output JSON format.
//
// As mentioned in the top-level documentation, there are two main ways to output JSON-formatted
// logs:
// 1. Using `with_json_formatter!` to override the global formatter.
// 2. Using `event!` to output a *single* JSON formatted log, with log level `Level::Event`.
fn main() {
    init!();
    // Use JSON format for all logs
    with_json_formatter!();

    // These lines will have JSON format
    info!(some_field = 1, "JSON formatted Info log");
    error!(some_field = 1, "JSON formatted Error log");

    while let Ok(()) = flush!() {}

    // Revert to default formatter
    with_formatter!(QuickLogFormatter);

    // These lines will have the default [utc datetime][log level]"message" format
    info!(some_field = 1, "Default formatted Info log");
    error!(some_field = 1, "Default formatted Error log");

    // `event!` is a special macro that *always* uses JSON formatting. It also has
    // the log level `Level::Event`.
    event!(some_field = 1, "Single JSON formatted log");

    while let Ok(()) = flush!() {}
}
