/// Used to amend which `Flush` is currently attached to `Quicklog`
/// An implementation can be passed in at runtime as long as it
/// adheres to the `Flush` trait in `quicklog-flush`
#[macro_export]
macro_rules! with_flush {
    ($flush:expr) => {{
        $crate::logger().use_flush($crate::make_container!($flush))
    }};
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
        $crate::Container::new($item)
    };
}

/// Calls `try_log` and unwraps result
#[doc(hidden)]
#[macro_export]
macro_rules! log {
  ($lvl:expr, $static_str:literal) => {
    $crate::try_log!($lvl, $static_str).unwrap();
  };

  ($lvl:expr, $static_str:literal, $($args:tt)+) => {
    $crate::try_log!($lvl, $static_str, $($args)+).unwrap();
  };
}

/// Checks if the current level we are trying to log is enabled by checking
/// static `MAX_LOG_LEVEL` which is evaluated at compile time
#[doc(hidden)]
#[macro_export]
macro_rules! is_level_enabled {
    ($level:expr) => {
        $level as usize >= $crate::level::MAX_LOG_LEVEL as usize
    };
}

/// Internal API that runs log and returns a Result, matches either a literal
/// or a literal with some arguments.
///
/// For the up to the first 10 args, references are supported if the reference implements [`Clone`]
///
/// If there are more than 10 args that are references, the user has to manually call `.clone()` on all arguments
/// that are references.
#[doc(hidden)]
#[macro_export]
macro_rules! try_log {
  ($lvl:expr, $static_str:literal) => {{
    use $crate::{Log, callsite::Callsite, clone_sender};
    use once_cell::sync::Lazy;

    if $crate::is_level_enabled!($lvl) {
      static CALLSITE: Lazy<Callsite> = Lazy::new(|| Callsite::new(clone_sender()));

      let log_line = $crate::make_container!(
        lazy_format::lazy_format!("[{}]\t{}", $lvl, $static_str));

      $crate::logger().log(
        &CALLSITE,
        log_line
      )
    } else {
      Ok(())
    }
  }};

  //TODO: There are issues passing in reference arguments of different lifetimes.
  // As a temporary fix, the macro for up to 10 arguments are handwritten. However,
  // there should be a better way to do this... Perhaps writing some recursive macro
  // to expand and name the arguments automatically? However, this works for now to
  // continue making progress
  ($lvl:expr, $static_str:literal, $arg:expr) => {{
    if $crate::is_level_enabled!($lvl) {
      use $crate::{Log, callsite::Callsite, clone_sender};
      use once_cell::sync::Lazy;

      static CALLSITE: Lazy<Callsite> = Lazy::new(|| Callsite::new(clone_sender()));

      let owned = $arg.to_owned();

      // TODO: Figure out a way to remove the double lazy_format! wrapping if possible
      // though, measurements have shown that this double wrapping does not lead to
      // a significant increase in run-time.
      let lazy_format_string = lazy_format::lazy_format!($static_str, owned);
      let log_line = $crate::make_container!(
        lazy_format::lazy_format!("[{}]\t{}", $lvl, lazy_format_string));

      $crate::logger().log(
        &CALLSITE,
        log_line
      )
    } else {
      Ok(())
    }
  }};

  ($lvl:expr, $static_str:literal, $arg1:expr, $arg2:expr) => {{
    if $crate::is_level_enabled!($lvl) {
      use $crate::{Log, callsite::Callsite, clone_sender};
      use once_cell::sync::Lazy;

      static CALLSITE: Lazy<Callsite> = Lazy::new(|| Callsite::new(clone_sender()));

      let owned_arg1 = $arg1.to_owned();
      let owned_arg2 = $arg2.to_owned();

      // TODO: Figure out a way to remove the double lazy_format! wrapping if possible
      // though, measurements have shown that this double wrapping does not lead to
      // a significant increase in run-time.
      let lazy_format_string = lazy_format::lazy_format!($static_str, owned_arg1, owned_arg2);
      let log_line = $crate::make_container!(
        lazy_format::lazy_format!("[{}]\t{}", $lvl, lazy_format_string));

      $crate::logger().log(
        &CALLSITE,
        log_line
      )
    } else {
      Ok(())
    }
  }};

  ($lvl:expr, $static_str:literal, $arg1:expr, $arg2:expr, $arg3:expr) => {{
    if $crate::is_level_enabled!($lvl) {
      use $crate::{Log, callsite::Callsite, clone_sender};
      use once_cell::sync::Lazy;

      static CALLSITE: Lazy<Callsite> = Lazy::new(|| Callsite::new(clone_sender()));

      let owned_arg1 = $arg1.to_owned();
      let owned_arg2 = $arg2.to_owned();
      let owned_arg3 = $arg3.to_owned();

      // TODO: Figure out a way to remove the double lazy_format! wrapping if possible
      // though, measurements have shown that this double wrapping does not lead to
      // a significant increase in run-time.
      let lazy_format_string = lazy_format::lazy_format!($static_str, owned_arg1, owned_arg2, owned_arg3);
      let log_line = $crate::make_container!(
        lazy_format::lazy_format!("[{}]\t{}", $lvl, lazy_format_string));

      $crate::logger().log(
        &CALLSITE,
        log_line
      )
    } else {
      Ok(())
    }
  }};

  ($lvl:expr, $static_str:literal, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr) => {{
    if $crate::is_level_enabled!($lvl) {
      use $crate::{Log, callsite::Callsite, clone_sender};
      use once_cell::sync::Lazy;

      static CALLSITE: Lazy<Callsite> = Lazy::new(|| Callsite::new(clone_sender()));

      let owned_arg1 = $arg1.to_owned();
      let owned_arg2 = $arg2.to_owned();
      let owned_arg3 = $arg3.to_owned();
      let owned_arg4 = $arg4.to_owned();

      // TODO: Figure out a way to remove the double lazy_format! wrapping if possible
      // though, measurements have shown that this double wrapping does not lead to
      // a significant increase in run-time.
      let lazy_format_string = lazy_format::lazy_format!($static_str, owned_arg1, owned_arg2, owned_arg3, owned_arg4);
      let log_line = $crate::make_container!(
        lazy_format::lazy_format!("[{}]\t{}", $lvl, lazy_format_string));

      $crate::logger().log(
        &CALLSITE,
        log_line
      )
    } else {
      Ok(())
    }
  }};

  ($lvl:expr, $static_str:literal, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr) => {{
    if $crate::is_level_enabled!($lvl) {
      use $crate::{Log, callsite::Callsite, clone_sender};
      use once_cell::sync::Lazy;

      static CALLSITE: Lazy<Callsite> = Lazy::new(|| Callsite::new(clone_sender()));

      let owned_arg1 = $arg1.to_owned();
      let owned_arg2 = $arg2.to_owned();
      let owned_arg3 = $arg3.to_owned();
      let owned_arg4 = $arg4.to_owned();
      let owned_arg5 = $arg5.to_owned();

      // TODO: Figure out a way to remove the double lazy_format! wrapping if possible
      // though, measurements have shown that this double wrapping does not lead to
      // a significant increase in run-time.
      let lazy_format_string = lazy_format::lazy_format!($static_str, owned_arg1, owned_arg2, owned_arg3, owned_arg4, owned_arg5);
      let log_line = $crate::make_container!(
        lazy_format::lazy_format!("[{}]\t{}", $lvl, lazy_format_string));

      $crate::logger().log(
        &CALLSITE,
        log_line
      )
    } else {
      Ok(())
    }
  }};

  ($lvl:expr, $static_str:literal, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr, $arg6:expr) => {{
    if $crate::is_level_enabled!($lvl) {
      use $crate::{Log, callsite::Callsite, clone_sender};
      use once_cell::sync::Lazy;

      static CALLSITE: Lazy<Callsite> = Lazy::new(|| Callsite::new(clone_sender()));

      let owned_arg1 = $arg1.to_owned();
      let owned_arg2 = $arg2.to_owned();
      let owned_arg3 = $arg3.to_owned();
      let owned_arg4 = $arg4.to_owned();
      let owned_arg5 = $arg5.to_owned();
      let owned_arg6 = $arg6.to_owned();

      // TODO: Figure out a way to remove the double lazy_format! wrapping if possible
      // though, measurements have shown that this double wrapping does not lead to
      // a significant increase in run-time.
      let lazy_format_string = lazy_format::lazy_format!($static_str, owned_arg1, owned_arg2, owned_arg3, owned_arg4, owned_arg5, owned_arg6);
      let log_line = $crate::make_container!(
        lazy_format::lazy_format!("[{}]\t{}", $lvl, lazy_format_string));

      $crate::logger().log(
        &CALLSITE,
        log_line
      )
    } else {
      Ok(())
    }
  }};

  ($lvl:expr, $static_str:literal, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr, $arg6:expr, $arg7:expr) => {{
    if $crate::is_level_enabled!($lvl) {
      use $crate::{Log, callsite::Callsite, clone_sender};
      use once_cell::sync::Lazy;

      static CALLSITE: Lazy<Callsite> = Lazy::new(|| Callsite::new(clone_sender()));

      let owned_arg1 = $arg1.to_owned();
      let owned_arg2 = $arg2.to_owned();
      let owned_arg3 = $arg3.to_owned();
      let owned_arg4 = $arg4.to_owned();
      let owned_arg5 = $arg5.to_owned();
      let owned_arg6 = $arg6.to_owned();
      let owned_arg7 = $arg7.to_owned();

      // TODO: Figure out a way to remove the double lazy_format! wrapping if possible
      // though, measurements have shown that this double wrapping does not lead to
      // a significant increase in run-time.
      let lazy_format_string = lazy_format::lazy_format!($static_str, owned_arg1, owned_arg2, owned_arg3, owned_arg4, owned_arg5, owned_arg6, owned_arg7);
      let log_line = $crate::make_container!(
        lazy_format::lazy_format!("[{}]\t{}", $lvl, lazy_format_string));

      $crate::logger().log(
        &CALLSITE,
        log_line
      )
    } else {
      Ok(())
    }
  }};

  ($lvl:expr, $static_str:literal, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr, $arg6:expr, $arg7:expr, $arg8:expr) => {{
    if $crate::is_level_enabled!($lvl) {
      use $crate::{Log, callsite::Callsite, clone_sender};
      use once_cell::sync::Lazy;

      static CALLSITE: Lazy<Callsite> = Lazy::new(|| Callsite::new(clone_sender()));

      let owned_arg1 = $arg1.to_owned();
      let owned_arg2 = $arg2.to_owned();
      let owned_arg3 = $arg3.to_owned();
      let owned_arg4 = $arg4.to_owned();
      let owned_arg5 = $arg5.to_owned();
      let owned_arg6 = $arg6.to_owned();
      let owned_arg7 = $arg7.to_owned();
      let owned_arg8 = $arg8.to_owned();

      // TODO: Figure out a way to remove the double lazy_format! wrapping if possible
      // though, measurements have shown that this double wrapping does not lead to
      // a significant increase in run-time.
      let lazy_format_string = lazy_format::lazy_format!($static_str, owned_arg1, owned_arg2, owned_arg3, owned_arg4, owned_arg5, owned_arg6, owned_arg7, owned_arg8);
      let log_line = $crate::make_container!(
        lazy_format::lazy_format!("[{}]\t{}", $lvl, lazy_format_string));

      $crate::logger().log(
        &CALLSITE,
        log_line
      )
    } else {
      Ok(())
    }
  }};

  ($lvl:expr, $static_str:literal, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr, $arg6:expr, $arg7:expr, $arg8:expr, $arg9:expr) => {{
    if $crate::is_level_enabled!($lvl) {
      use $crate::{Log, callsite::Callsite, clone_sender};
      use once_cell::sync::Lazy;

      static CALLSITE: Lazy<Callsite> = Lazy::new(|| Callsite::new(clone_sender()));

      let owned_arg1 = $arg1.to_owned();
      let owned_arg2 = $arg2.to_owned();
      let owned_arg3 = $arg3.to_owned();
      let owned_arg4 = $arg4.to_owned();
      let owned_arg5 = $arg5.to_owned();
      let owned_arg6 = $arg6.to_owned();
      let owned_arg7 = $arg7.to_owned();
      let owned_arg8 = $arg8.to_owned();
      let owned_arg9 = $arg9.to_owned();

      // TODO: Figure out a way to remove the double lazy_format! wrapping if possible
      // though, measurements have shown that this double wrapping does not lead to
      // a significant increase in run-time.
      let lazy_format_string = lazy_format::lazy_format!($static_str, owned_arg1, owned_arg2, owned_arg3, owned_arg4, owned_arg5, owned_arg6, owned_arg7, owned_arg8, owned_arg9);
      let log_line = $crate::make_container!(
        lazy_format::lazy_format!("[{}]\t{}", $lvl, lazy_format_string));

      $crate::logger().log(
        &CALLSITE,
        log_line
      )
    } else {
      Ok(())
    }
  }};

  ($lvl:expr, $static_str:literal, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr, $arg6:expr, $arg7:expr, $arg8:expr, $arg9:expr, $arg10:expr) => {{
    if $crate::is_level_enabled!($lvl) {
      use $crate::{Log, callsite::Callsite, clone_sender};
      use once_cell::sync::Lazy;

      static CALLSITE: Lazy<Callsite> = Lazy::new(|| Callsite::new(clone_sender()));

      let owned_arg1 = $arg1.to_owned();
      let owned_arg2 = $arg2.to_owned();
      let owned_arg3 = $arg3.to_owned();
      let owned_arg4 = $arg4.to_owned();
      let owned_arg5 = $arg5.to_owned();
      let owned_arg6 = $arg6.to_owned();
      let owned_arg7 = $arg7.to_owned();
      let owned_arg8 = $arg8.to_owned();
      let owned_arg9 = $arg9.to_owned();
      let owned_arg10 = $arg10.to_owned();

      // TODO: Figure out a way to remove the double lazy_format! wrapping if possible
      // though, measurements have shown that this double wrapping does not lead to
      // a significant increase in run-time.
      let lazy_format_string = lazy_format::lazy_format!($static_str, owned_arg1, owned_arg2, owned_arg3, owned_arg4, owned_arg5, owned_arg6, owned_arg7, owned_arg8, owned_arg9, owned_arg10);
      let log_line = $crate::make_container!(
        lazy_format::lazy_format!("[{}]\t{}", $lvl, lazy_format_string));

      $crate::logger().log(
        &CALLSITE,
        log_line
      )
    } else {
      Ok(())
    }
  }};

  ($lvl:expr, $static_str:literal, $($args:tt)+) => {{
    use $crate::{Log, callsite::Callsite, clone_sender};
    use once_cell::sync::Lazy;

    if $crate::is_level_enabled!($lvl) {
      static CALLSITE: Lazy<Callsite> = Lazy::new(|| Callsite::new(clone_sender()));

      // TODO: Figure out a way to remove the double lazy_format! wrapping if possible
      // though, measurements have shown that this double wrapping does not lead to
      // a significant increase in run-time.
      let lazy_format_string = lazy_format::lazy_format!($static_str, $($args)+);
      let log_line = $crate::make_container!(
        lazy_format::lazy_format!("[{}]\t{}", $lvl, lazy_format_string));

      $crate::logger().log(
        &CALLSITE,
        log_line
      )
    } else {
      Ok(())
    }
  }};
}

/// Allows flushing onto an implementor of [`Flush`], which can be modified with
/// [`with_flush!`] macro, and passing in of a timeout.
///
/// [`Flush`]: `quicklog_flush::Flush`
#[macro_export]
macro_rules! try_flush_with_timeout {
    ($timeout:expr) => {{
        use $crate::Log;
        $crate::logger().flush(Some(timeout))
    }};
}

/// Allows flushing onto an implementor of [`Flush`], which can be modified with
/// [`with_flush!`] macro.
///
/// [`Flush`]: `quicklog_flush::Flush`
#[macro_export]
macro_rules! try_flush {
    () => {{
        use $crate::Log;
        $crate::logger().flush(None)
    }};
}

/// Allows flushing onto an implementor of [`Flush`], which can be modified with
/// [`with_flush!`] macro, simply unwrapped from [`try_flush!`]
///
/// [`Flush`]: `quicklog_flush::Flush`
#[macro_export]
macro_rules! flush {
    () => {
        $crate::try_flush!().unwrap();
    };
}

/// Allows flushing onto an implementor of [`Flush`], which can be modified with
/// [`with_flush!`] macro, and allows passing in of a timeout, simply unwrapped
/// from [`try_flush_with_timeout!`].
///
///
/// [`Flush`]: `quicklog_flush::Flush`
#[macro_export]
macro_rules! flush_with_timeout {
    ($timeout:expr) => {
        $crate::try_flush_with_timeout!($timeout).unwrap();
    };
}

/// Trace level log
#[macro_export]
macro_rules! trace {
  {$static_str:literal} => ( $crate::log!($crate::level::Level::Trace, $static_str) );
  {$static_str:literal, $($args:tt)+} => ( $crate::log!($crate::level::Level::Trace, $static_str, $($args)*) );
}

/// Debug level log
#[macro_export]
macro_rules! debug {
  {$static_str:literal} => ( $crate::log!($crate::level::Level::Debug, $static_str) );
  {$static_str:literal, $($args:tt)+} => ( $crate::log!($crate::level::Level::Debug, $static_str, $($args)*) );
}

/// Info level log
#[macro_export]
macro_rules! info {
  {$static_str:literal} => ( $crate::log!($crate::level::Level::Info, $static_str) );
  {$static_str:literal, $($args:tt)+} => ( $crate::log!($crate::level::Level::Info, $static_str, $($args)*) );
}

/// Warn level log
#[macro_export]
macro_rules! warn {
  {$static_str:literal} => ( $crate::log!($crate::level::Level::Warn, $static_str) );
  {$static_str:literal, $($args:tt)+} => ( $crate::log!($crate::level::Level::Warn, $static_str, $($args)*) );
}

/// Error level log
#[macro_export]
macro_rules! error {
  {$static_str:literal} => ( $crate::log!($crate::level::Level::Error, $static_str) );
  {$static_str:literal, $($args:tt)+} => ( $crate::log!($crate::level::Level::Error, $static_str, $($args)*) );
}
