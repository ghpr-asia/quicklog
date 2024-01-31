use quicklog::{
    debug, error, flush, info, level::LevelFilter, set_max_level, target::TargetFilter, trace,
    warn, with_target_filter, FlushError,
};

mod common;

#[cfg(feature = "target-filter")]
mod my_module {
    use super::*;

    pub fn info_log_in_module() {
        info!("Info log from `my_module`!");
    }

    pub fn error_log_in_module() {
        error!("Error log from `my_module`!");
    }
}

#[test]
fn target_filter() {
    setup!();
    // log all Info logs by default
    set_max_level(LevelFilter::Info);

    // specific log filters
    let target_filter = TargetFilter::default()
        .with_target("filter::my_module", LevelFilter::Error)
        .with_target("inner", LevelFilter::Off);
    with_target_filter!(target_filter);

    trace!("hello world");
    assert_eq!(flush!().unwrap_err(), FlushError::Empty);
    debug!("hello world");
    assert_eq!(flush!().unwrap_err(), FlushError::Empty);

    info!("hello world");
    assert!(flush!().is_ok());
    warn!("hello world");
    assert!(flush!().is_ok());
    error!("hello world");
    assert!(flush!().is_ok());

    #[cfg(feature = "target-filter")]
    {
        // this should not be visible
        my_module::info_log_in_module();
        assert_eq!(flush!().unwrap_err(), FlushError::Empty);

        // this should be visible
        my_module::error_log_in_module();
        assert!(flush!().is_ok());

        // all these should not be visible
        trace!(target: "inner", "hello world");
        assert_eq!(flush!().unwrap_err(), FlushError::Empty);
        debug!(target: "inner", "hello world");
        assert_eq!(flush!().unwrap_err(), FlushError::Empty);
        info!(target: "inner", "hello world");
        assert_eq!(flush!().unwrap_err(), FlushError::Empty);
        warn!(target: "inner", "hello world");
        assert_eq!(flush!().unwrap_err(), FlushError::Empty);
        error!(target: "inner", "hello world");
        assert_eq!(flush!().unwrap_err(), FlushError::Empty);
    }
}
