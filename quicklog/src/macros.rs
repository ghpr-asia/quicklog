/// Used to amend which [`Flush`](quicklog_flush::Flush) implementor is
/// currently attached to the global [`Quicklog`](crate::Quicklog) logger.
#[macro_export]
macro_rules! with_flush {
    ($flush:expr) => {{
        $crate::logger().use_flush(std::boxed::Box::new($flush))
    }};
}

/// Used to amend which [`PatternFormatter`](crate::PatternFormatter)
/// implementor is currently attached to the global
/// [`Quicklog`](crate::Quicklog) logger.
#[macro_export]
macro_rules! with_formatter {
    ($formatter:expr) => {{
        $crate::logger().use_formatter(std::boxed::Box::new($formatter))
    }};
}

/// Overwrites the [`Flush`](quicklog_flush::Flush)
/// implementor in [`Quicklog`](crate::Quicklog) with a
/// [`FileFlusher`](quicklog_flush::file_flusher::FileFlusher) using the
/// provided file path.
#[macro_export]
macro_rules! with_flush_into_file {
    ($file_path:expr) => {{
        use quicklog_flush::FileFlusher;
        let flusher = FileFlusher::new($file_path);
        $crate::logger().use_flush(std::boxed::Box::new(flusher));
    }};
}

/// Initializes Quicklog by calling [`Quicklog::init()`]. **NOTE**: This should
/// only be called once in the application!
///
/// [`Quicklog::init()`]: crate::Quicklog::init
#[macro_export]
macro_rules! init {
    () => {
        $crate::Quicklog::init();
    };
}

/// Used to amend which [`Clock`](quicklog_clock::Clock) implementor is
/// currently attached to the global [`Quicklog`](crate::Quicklog) logger.
#[macro_export]
macro_rules! with_clock {
    ($clock:expr) => {{
        $crate::logger().use_clock(std::boxed::Box::new($clock))
    }};
}

/// Checks if the current level we are trying to log is enabled
#[doc(hidden)]
#[macro_export]
macro_rules! is_level_enabled {
    ($level:expr) => {
        $level as usize >= $crate::level::max_level() as usize
    };
}

/// Flushes all log records onto an implementor of [`Flush`], which can be
/// modified with [`with_flush!`] macro.
///
/// [`Flush`]: `quicklog_flush::Flush`
#[macro_export]
macro_rules! flush {
    () => {
        $crate::logger().flush().unwrap();
    };
}
