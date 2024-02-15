use quicklog::{config, error, flush, formatter, info, init};

// Demonstrates customization of log output destination and formatting.
fn main() {
    // Use JSON format for all logs and append logs to "quicklog.log". Note that
    // the file is created if it does not exist.
    let config = config()
        .formatter(formatter().json().build())
        .file_flusher("quicklog.log");
    init!(config);

    info!(some_field = 1, "Info to file");
    error!(some_field = 1, "Error to file");

    while let Ok(()) = flush!() {}
}
