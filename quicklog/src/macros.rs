/// Used to amend which [`Flush`](crate::Flush) implementor is
/// currently attached to the global [`Quicklog`](crate::Quicklog) logger.
///
/// By default, logs are flushed to stdout. See also the [top-level
/// documentation](crate#flush) for information on defining your own flushers.
#[macro_export]
macro_rules! with_flush {
    ($flush:expr) => {{
        $crate::logger().use_flush(std::boxed::Box::new($flush))
    }};
}

/// Used to amend which [`PatternFormatter`](crate::fmt::PatternFormatter)
/// implementor is currently attached to the global
/// [`Quicklog`](crate::Quicklog) logger.
///
/// By default, logs are formatted with the format `[utc
/// datetime][log level]"message`. See also the [top-level
/// documentation](crate#patternformatter) for information on defining your own
/// formatters.
#[macro_export]
macro_rules! with_formatter {
    ($formatter:expr) => {{
        $crate::logger().use_formatter(std::boxed::Box::new($formatter))
    }};
}

/// Sets a [`TargetFilter`](crate::target::TargetFilter) on the global logger.
///
/// This filters out logs at runtime based on their target and the log level
/// filter attached to it.
#[macro_export]
macro_rules! with_target_filter {
    ($filter:expr) => {{
        $crate::logger().with_target_filter($filter)
    }};
}

/// Overwrites the [`Flush`](crate::Flush)
/// implementor in [`Quicklog`](crate::Quicklog) with a
/// [`FileFlusher`](crate::FileFlusher) using the
/// provided file path.
///
/// By default, logs are flushed to stdout. See also the [top-level
/// documentation](crate#flush) for information on defining your own flushers.
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
/// Can optionally be called with a `usize` argument indicating the desired size
/// of the backing logging queue. Defaults to 1MB otherwise. Note that this
/// size may be rounded up or adjusted for better performance. See also the
/// [top-level documentation](crate#configuration-of-max-logging-capacity).
///
/// # Examples
///
/// ```rust no_run
/// use quicklog::init;
///
/// # fn main() {
/// init!();
/// # }
/// ```
///
/// ```rust no_run
/// use quicklog::init;
///
/// # fn main() {
/// // 8MB
/// init!(8 * 1024 * 1024);
/// # }
/// ```
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

/// Flushes a single log record onto an implementor of [`Flush`], which can be
/// modified with [`with_flush!`] macro.
///
/// [`Flush`]: `crate::Flush`
///
/// # Examples
///
/// ```rust
/// use quicklog::{flush, info, init};
///
/// # fn main() {
/// init!();
/// info!("Hello from the other side: {}", "bye");
///
/// assert!(flush!().is_ok());
/// # }
/// ```
#[macro_export]
macro_rules! flush {
    () => {
        $crate::logger().flush()
    };
}

/// Commits all written log records to be available for reading.
///
/// # Examples
///
/// ```rust
/// use quicklog::{commit, flush, info_defer, init};
///
/// # fn main() {
/// init!();
/// info_defer!(payload = 123, "Some message: {:^}", "hello world!");
///
/// // do some work..
///
/// // make previously logged message available for flushing
/// commit!();
///
/// assert!(flush!().is_ok());
/// # }
/// ```
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
