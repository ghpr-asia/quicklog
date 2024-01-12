use quicklog::{debug, error, flush, info, init, level::LevelFilter, set_max_level, trace, warn};

// Shows how log records can be filtered out at runtime.
fn main() {
    init!();

    // The default `LevelFilter` is `Trace`, so all logs will be recorded
    trace!("Trace");
    debug!("Debug");
    info!("Info");
    warn!("Warn");
    error!("Error");

    // Change filter to only errors at runtime
    set_max_level(LevelFilter::Error);

    // Now all these should not be visible...
    trace!("Trace 2");
    debug!("Debug 2");
    info!("Info 2");
    warn!("Warn 2");

    // ...but errors are.
    error!("Error 2");

    while let Ok(()) = flush!() {}
}
