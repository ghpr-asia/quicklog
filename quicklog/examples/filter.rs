use quicklog::{
    debug, error, flush, formatter, info, init, level::LevelFilter, set_max_level,
    target::TargetFilter, trace, warn, with_target_filter,
};

mod my_module {
    use super::*;

    pub fn info_log_in_module() {
        info!("Info log from `my_module`!");
    }

    pub fn error_log_in_module() {
        error!("Error log from `my_module`!");
    }
}

// Shows how log records can be filtered out at runtime.
//
// Ensure that the `target-filter` feature is enabled to see the effects of
// target-based filtering as well.
fn main() {
    init!();
    formatter().with_target(true).init();

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

    // Reset global default to `Info`
    set_max_level(LevelFilter::Info);
    let target_filter =
        TargetFilter::default().with_target("filter::my_module", LevelFilter::Error);
    with_target_filter!(target_filter);

    // with the `target-filter` feature, this should not be visible
    my_module::info_log_in_module();

    // this should be visible
    my_module::error_log_in_module();

    // remaining logs that don't have a target filter specified (i.e. in `filter` module) should
    // still be logged
    info!("Info 3");

    while let Ok(()) = flush!() {}
}
