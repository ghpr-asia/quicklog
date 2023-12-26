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

/// Commits all written log records to be available for reading.
///
/// This ensures that logs are committed at the end of the current scope *even
/// if it is exited early*. This could be due to an error being thrown,
/// for instance.
///
/// # Examples
///
/// ```rust no_run
/// use quicklog::{commit_on_scope_end, info_defer};
/// # enum IntError {
/// #     WrongInt
/// # }
///
/// fn possible_err(some_val: usize) -> Result<(), IntError> {
///     info_defer!("Entered possible_err with value: {:^}", some_val);
///     // will always call `commit!` when the current scope ends, i.e. when
///     // the function returns
///     commit_on_scope_end!();
///
///     // hot path: perform some computations
///     // ...
///
///     // possible error path: function will exit without calling `commit!`
///     if some_val < 5 {
///         return Err(IntError::WrongInt);
///     }
///
///     // commit here might not be reached!
///     // commit!();
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! commit_on_scope_end {
    () => {
        let ___x = $crate::__macro_helpers::CommitOnDrop;
    };
}
