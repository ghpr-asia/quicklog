use quicklog::{
    config, debug, error, flush, formatter, info, init, level::LevelFilter, set_max_level,
    target::TargetFilter, trace, warn,
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
    let target_filter =
        TargetFilter::default().with_target("filter::my_module", LevelFilter::Error);
    init!(config()
        .formatter(formatter().with_target(true).build())
        .target_filter(target_filter));

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

    // with the `target-filter` feature, this should not be visible
    my_module::info_log_in_module();

    // this should be visible
    my_module::error_log_in_module();

    // remaining logs that don't have a target filter specified (i.e. in `filter` module) should
    // still adhere to the global level, which is still `Error` in this case (as set by
    // `set_max_level` above).
    //
    // hence, this should not be visible again
    info!("Info 3");

    while let Ok(()) = flush!() {}
}
