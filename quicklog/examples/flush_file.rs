use quicklog::{error, flush, info, init, with_flush_into_file, with_json_formatter};

// Demonstrates customization of log output destination and formatting.
fn main() {
    init!();
    // Use JSON format for all logs
    with_json_formatter!();

    // These lines will be logged to stdout, by default
    info!(some_field = 1, "Info to stdout");
    error!(some_field = 1, "Error to stdout");

    // Note that `flush!` needs to be called to log to stdout before the output
    // destination is changed to `quicklog.log` below.
    //
    // This is because `quicklog` holds onto the two previous logs in memory
    // and only decides where to output them when `flush!` is called. If we
    // commented out the immediately following `while let Ok(()) = flush!() {}`,
    // and only perform a flush at the very end of `main`, then *all* records,
    // including the ones above, will be flushed into `quicklog.log`.
    while let Ok(()) = flush!() {}

    // These lines will be appended to `quicklog.log`, creating it if it does
    // not exist.
    with_flush_into_file!("quicklog.log");
    info!(some_field = 1, "Info to file");
    error!(some_field = 1, "Error to file");

    while let Ok(()) = flush!() {}
}
