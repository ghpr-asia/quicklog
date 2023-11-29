/// Used to amend which [`Flush`](crate::Flush) implementor is
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

#[doc(hidden)]
#[macro_export]
macro_rules! str_format {
    ($in:expr, $fmt_str:expr, $($args:tt)*) => {{
        use ::std::fmt::Write;
        let mut s = quicklog::BumpString::with_capacity_in(2048, $in);
        s.write_fmt(format_args!($fmt_str, $($args)*)).unwrap();
        s
    }};

    ($in:expr, $fmt_str:expr) => {{
        use ::std::fmt::Write;
        let mut s = quicklog::BumpString::with_capacity_in(2048, $in);
        s.write_fmt(format_args!($fmt_str)).unwrap();
        s
    }};
}
