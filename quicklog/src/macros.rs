/// Used to amend which [`Flush`](crate::Flush) implementor is
/// currently attached to the global [`Quicklog`](crate::Quicklog) logger.
#[macro_export]
macro_rules! with_flush {
    ($flush:expr) => {{
        $crate::logger().use_flush(std::boxed::Box::new($flush))
    }};
}

/// Used to amend which [`PatternFormatter`](crate::formatter::PatternFormatter)
/// implementor is currently attached to the global
/// [`Quicklog`](crate::Quicklog) logger.
#[macro_export]
macro_rules! with_formatter {
    ($formatter:expr) => {{
        $crate::logger().use_formatter(std::boxed::Box::new($formatter))
    }};
}

/// Sets the [`JsonFormatter`](crate::formatter::JsonFormatter) as the global
/// default.
#[macro_export]
macro_rules! with_json_formatter {
    () => {
        $crate::logger().use_formatter(std::boxed::Box::new($crate::formatter::JsonFormatter))
    };
}

/// Overwrites the [`Flush`](crate::Flush)
/// implementor in [`Quicklog`](crate::Quicklog) with a
/// [`FileFlusher`](crate::FileFlusher) using the
/// provided file path.
#[macro_export]
macro_rules! with_flush_into_file {
    ($file_path:expr) => {{
        use $crate::FileFlusher;
        let flusher = FileFlusher::new($file_path);
        $crate::logger().use_flush(std::boxed::Box::new(flusher));
    }};
}

/// Initializes Quicklog by calling [`Quicklog::init()`]. You should only need
/// to call this once in the application.
///
/// [`Quicklog::init()`]: crate::Quicklog::init
#[macro_export]
macro_rules! init {
    () => {
        $crate::Quicklog::init();
    };
    ($capacity:expr) => {
        $crate::Quicklog::init_with_capacity($capacity);
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

/// Flushes a single log record onto an implementor of [`Flush`], which can be
/// modified with [`with_flush!`] macro.
///
/// [`Flush`]: `crate::Flush`
#[macro_export]
macro_rules! flush {
    () => {
        $crate::logger().flush()
    };
}

/// Commits all written log records to be available for reading.
#[macro_export]
macro_rules! commit {
    () => {
        $crate::logger().commit_write();
    };
}
