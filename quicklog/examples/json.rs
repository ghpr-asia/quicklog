use quicklog::{config, error, event, flush, formatter, info, init};

// Demonstrates how to output JSON format.
//
// As mentioned in the top-level documentation, there are two main ways to output JSON-formatted
// logs:
// 1. Configuring the default formatter using the provided `formatter` builder
// 2. Using `event!` to output a *single* JSON formatted log, with log level `Level::Event`.
fn main() {
    // Use JSON format for all logs
    init!(config().formatter(formatter().json().build()));

    // These lines will have JSON format
    info!(some_field = 1, "JSON formatted Info log");
    error!(some_field = 1, "JSON formatted Error log");

    while let Ok(()) = flush!() {}

    // `event!` is a special macro that *always* uses JSON formatting. It also has
    // the log level `Level::Event`.
    event!(some_field = 1, "Single JSON formatted log");

    while let Ok(()) = flush!() {}
}
