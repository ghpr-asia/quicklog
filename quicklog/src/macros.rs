/// Initializes Quicklog by calling [`Quicklog::init()`]. You should only need
/// to call this once in the application.
///
/// Can optionally be called with a [`Config`](crate::Config) to pass custom options
/// to the default logger.
///
/// # Examples
///
/// Using default configuration:
///
/// ```rust
/// use quicklog::init;
///
/// # fn main() {
/// init!();
/// # }
/// ```
///
/// Using custom configuration:
///
/// ```rust
/// use quicklog::{config, init};
///
/// # fn main() {
/// // 8MB queue capacity
/// let config = config().capacity(8 * 1024 * 1024);
/// init!(config);
/// # }
/// ```
///
/// [`Quicklog::init()`]: crate::Quicklog::init
#[macro_export]
macro_rules! init {
    () => {
        $crate::Quicklog::init();
    };
    ($config:expr) => {
        $crate::Quicklog::init_with_config($config);
    };
}

/// Flushes a single log record onto an implementor of [`Flush`].
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
