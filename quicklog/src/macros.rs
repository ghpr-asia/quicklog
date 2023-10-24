/// Used to amend which `Flush` is currently attached to `Quicklog`
/// An implementation can be passed in at runtime as long as it
/// adheres to the `Flush` trait in `quicklog-flush`
#[macro_export]
macro_rules! with_flush {
    ($flush:expr) => {{
        $crate::logger().use_flush($crate::make_container!($flush))
    }};
}

/// Used to amend which `PatternFormatter` is currently attached to `Quicklog`
/// An implementation can be passed in at runtime as long as it
/// adheres to the `PatternFormatter` trait in `quicklog-formatter`
#[macro_export]
macro_rules! with_formatter {
    ($formatter:expr) => {{
        $crate::logger().use_formatter($crate::make_container!($formatter))
    }};
}

/// Flushes log lines into the file path specified
#[macro_export]
macro_rules! with_flush_into_file {
    ($file_path:expr) => {{
        use quicklog_flush::FileFlusher;
        let flusher = FileFlusher::new($file_path);
        $crate::logger().use_flush($crate::make_container!(flusher));
    }};
}

/// Initializes Quicklog by calling [`Quicklog::init()`]
/// Should only be called once in the application
///
/// [`Quicklog::init()`]: crate::Quicklog::init
#[macro_export]
macro_rules! init {
    () => {
        $crate::logger().init();
    };
}

/// Used to amend which `Clock` is currently attached to `Quicklog`
/// An implementation can be passed in at runtime as long as it
/// adheres to the `Clock` trait in `quicklog-clock`
#[macro_export]
macro_rules! with_clock {
    ($clock:expr) => {{
        $crate::logger().use_clock($crate::make_container!($clock))
    }};
}

/// Wrapper to wrap an item inside of the Container currently used
/// by Quicklog, not meant for external use
#[doc(hidden)]
#[macro_export]
macro_rules! make_container {
    ($item:expr) => {
        std::boxed::Box::new($item)
    };
}

/// Checks if the current level we are trying to log is enabled
#[doc(hidden)]
#[macro_export]
macro_rules! is_level_enabled {
    ($level:expr) => {
        $level as usize >= $crate::level::max_level() as usize
    };
}

// in debug, without clone, we have to make a Arc of Store, this ensures
// we are able to properly keep track of the stores we are using
//
// in release, we have a clonable store, so we remove the overhead of Arc
#[doc(hidden)]
#[macro_export]
macro_rules! make_store {
    ($serializable:expr) => {{
        let (store, _) = $serializable
            .encode($crate::logger().get_chunk_as_mut($serializable.buffer_size_required()));

        store
    }};
}

/// Allows flushing onto an implementor of [`Flush`], which can be modified with
/// [`with_flush!`] macro and returns [`RecvResult`]
///
/// [`Flush`]: quicklog_flush::Flush
/// [`RecvResult`]: crate::RecvResult
#[macro_export]
macro_rules! try_flush {
    () => {{
        use $crate::Log;
        $crate::logger().flush_one()
    }};
}

/// Allows flushing onto an implementor of [`Flush`], which can be modified with
/// [`with_flush!`] macro and unwraps and ignores errors from [`try_flush`]
///
/// [`Flush`]: `quicklog_flush::Flush`
#[macro_export]
macro_rules! flush {
    () => {
        $crate::try_flush!().unwrap_or(());
    };
}
