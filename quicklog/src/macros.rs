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

/// Calls `try_log` and discards any errors
#[macro_export]
macro_rules! log {
  ($lvl:expr, $static_str:literal) => {
    $crate::try_log!($lvl, $static_str).unwrap_or(())
  };

  ($lvl:expr, $static_str:literal, $($args:tt)*) => {
    $crate::try_log!($lvl, $static_str, $($args)*).unwrap_or(())
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
        use $crate::serialize::Serialize;
        $serializable
            .encode($crate::logger().get_chunk_as_mut($serializable.buffer_size_required()))
    }};
}

/// Runs log and returns a Result, matches either a literal / or a literal with some arguments
/// which are matched recursively.
#[macro_export]
macro_rules! try_log {
  // === no args
  ($lvl:expr, $static_str:literal) => {{
    if $crate::is_level_enabled!($lvl) {
      use $crate::{Log, make_container};


      let log_record = $crate::LogRecord {
        level: $lvl,
        module_path:$crate::module_path!(),
        file: $crate::file!(),
        line: $crate::line!(),
        log_line: make_container!($static_str),
      };

      $crate::logger().log(log_record)
    } else {
      Ok(())
    }
  }};

  // === entry
  // starts recursion on normal arguments to be passed into log
  ($lvl:expr, $static_str:literal, $($args:tt)*) => {
    $crate::try_log!($lvl, $static_str @@ {{}} @ (x) () $($args)*)
  };

  // === base case
  // perform the logging by owning arguments
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($([$($field:tt)*])*)) => {
    $crate::paste::paste! {{
      if $crate::is_level_enabled!($lvl) {
        use $crate::{Log, make_container};

        // allow unused_parens for case with 1 single field
        #[allow(unused_parens)]
        let ($([<$($field)*>]),*) = ($(($args).to_owned()),*);

        let log_record = $crate::LogRecord {
            level: $lvl,
            module_path:$crate::module_path!(),
            file: $crate::file!(),
            line: $crate::line!(),
            log_line: make_container!($crate::lazy_format::make_lazy_format!(|f| {
              write!(f,  $static_str,  $([<$($field)*>]),*)
            })),
          };



        $crate::logger().log(log_record)
      } else {
        Ok(())
      }
    }}
  };

  // === recursive cases
  // recurses through the prefixes, adding a new 'x' character at each level and creating the idents
  // this recurses until `$($rest)*` is empty
  //
  // for each recursion, there's the recursive case, and the final case, with no more recursive params
  // - this is necessary since trying to do `$next:expr $(,)*` with optional commas doesn't work well
  // - we can't match `$(,)+` either, since then our final case wouldn't work properly
  //
  // references:
  // generating multiple arg names using `paste!`: https://stackoverflow.com/questions/33193846/using-macros-how-to-get-unique-names-for-struct-fields/74931885#74931885
  // matching arguments with `%`, `?`: tracing `valueset!` macro: https://github.com/tokio-rs/tracing/blob/998774eb7a9e8f5fe7020fa660fbcca9aaec2169/tracing/src/macros.rs#L2183
  //
  // there are 5 cases right now to match for the recursion, in the same order as written below:
  // 1. `literal     = expr` arg
  // 2. `$($ident).+ = expr` arg
  // for 1 and 2, there are 4 sub cases:
  //    a. &expr - reference argument where we need to clone
  //    b. %expr - eager format into Display
  //    c. ?expr - eager format into Debug
  //    d. ^expr - serialize
  //    e.  expr - no special handling required
  // 3. no prefix  -  $next: own and pass to lazy_format
  // 4. `%` prefix - %$next: eagerly format object that implements Display
  // 5. `?` prefix - ?$next: eagerly format object that implements Debug
  // 6. `^` prefix - #$next: implemenets serialize trait, simply clone the Store

  // cases 1-2

  // case 1a: match `literal = &expr` argument, where argument is a reference
  // example: info!("some string field {}", "string field here" = &some_variable)
  // we need to own the argument before we can pass it into the lazy_format closure
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) $key:literal = &$next:expr, $($rest:tt)*) => {{
    let arg = (&$next).to_owned();
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , $crate::lazy_format::lazy_format!("{}={}", $key, arg) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]) $($rest)*)
  }};

  // case 1b: match `literal = %expr` argument, eagerly format expr into Display
  // example: info!("some string field {}", "string field here" = %some_variable)
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) $key:literal = %$next:expr, $($rest:tt)*) => {
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , format!("{}={}", $key, $next) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]) $($rest)*)
  };

  // case 1c: match `literal = ?expr` argument, eagerly format expr into Debug
  // example: info!("some string field {}", "string field here" = ?some_variable)
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) $key:literal = ?$next:expr, $($rest:tt)*) => {
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , format!("{}={:?}", $key, $next) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]) $($rest)*)
  };

  // case 1d: match `literal = ^expr` argument, where argument impls `Serialize`
  // example: info!("some string field {}", "string field here" = ^some_variable)
  // we need to own the argument before we can pass it into the lazy_format closure
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) $key:literal = ^$next:expr, $($rest:tt)*) => {{
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , $crate::lazy_format::lazy_format!("{}={}", $key, $crate::make_store!($next)) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]) $($rest)*)
  }};

  // case 1e: match `literal = expr` argument, normal argument pass by move
  // example: info!("some string field {}", "string field here" = some_variable)
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) $key:literal = $next:expr, $($rest:tt)*) => {
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , $crate::lazy_format::lazy_format!("{}={}", $key, $next) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]) $($rest)*)
  };

  // case 2a: match `ident.ident1.ident2 = &expr` argument, where expr represents a reference
  // example: info!("some nested ident {}", some.nested.field.of.idents = &some_expr)
  // we need to own the argument first
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) $($key:ident).+ = &$next:expr, $($rest:tt)*) => {{
    let arg = (&$next).to_owned();
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , $crate::lazy_format::lazy_format!("{}={}", stringify!($($key).+), arg) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]) $($rest)*)
  }};

  // case 2b: match `ident.ident1.ident2 = %expr` argument, eagerly format expr into Display
  // example: info!("some nested ident {}", some.nested.field.of.idents = %some_expr)
  // we need to own the argument first
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) $($key:ident).+ = %$next:expr, $($rest:tt)*) => {
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , format!("{}={}", stringify!($($key).+), $next) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]) $($rest)*)
  };

  // case 2c: match `ident.ident1.ident2 = ?expr` argument, eagerly format expr into Debug
  // example: info!("some nested ident {}", some.nested.field.of.idents = ?some_expr)
  // we need to own the argument first
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) $($key:ident).+ = ?$next:expr, $($rest:tt)*) => {
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , format!("{}={}", stringify!($($key).+), $next) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]) $($rest)*)
  };

  // case 2d: match `ident.ident1.ident2 = ^expr` argument, where argument implements Serialize
  // example: info!("some nested ident {}", some.nested.field.of.idents = ^some_expr)
  // we need to own the argument first
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) $($key:ident).+ = ^$next:expr, $($rest:tt)*) => {{
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , $crate::::lazy_format!("{}={}", stringify!($($key).+), $crate::make_store!($next)) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]) $($rest)*)
  }};

  // case 2e: match `ident.ident1.ident2 = expr` argument
  // example: info!("some nested ident {}", some.nested.field.of.idents = some_expr)
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) $($key:ident).+ = $next:expr, $($rest:tt)*) => {
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , $crate::lazy_format::lazy_format!("{}={}", stringify!($($key).+), $next) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]) $($rest)*)
  };

  // recursive cases 1-2

  // case 1a - ref &$next
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) $key:literal = &$next:expr) => {{
    let arg = (&$next).to_owned();
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , $crate::lazy_format::lazy_format!("{}={}", $key, arg) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]))
  }};

  // case 1b - %$next
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) $key:literal = %$next:expr) => {
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , format!("{}={}", $key, $next) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]))
  };

  // case 1c - ?$next
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) $key:literal = ?$next:expr) => {
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , format!("{}={:?}", $key, $next) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]))
  };

  // case 1d - ^$next
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) $key:literal = ^$next:expr) => {
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , $crate::lazy_format::lazy_format!("{}={}", $key, $crate::make_store!($next)) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]))
  };

  // case 1e - move $next
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) $key:literal = $next:expr) => {
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , $crate::lazy_format::lazy_format!("{}={}", $key, $next) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]))
  };

  // case 2a - ref &$next
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) $($key:ident).+ = &$next:expr) => {{
    let arg = (&$next).to_owned();
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , $crate::lazy_format::lazy_format!("{}={}", stringify!($($key).+), arg) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]))
  }};

  // case 2b - %$next
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) $($key:ident).+ = %$next:expr) => {
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , format!("{}={}", stringify!($($key).+), &$next) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]))
  };

  // case 2c - ?$next
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) $($key:ident).+ = ?$next:expr) => {
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , format!("{}={:?}", stringify!($($key).+), $next) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]))
  };

  // case 2d - ^$next
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) $($key:ident).+ = ^$next:expr) => {{
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , $crate::lazy_format::lazy_format!("{}={}", stringify!($($key).+), $crate::make_store!($next)) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]))
  }};

  // case 2e - move $next
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) $($key:ident).+ = $next:expr) => {
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , $crate::lazy_format::lazy_format!("{}={}", stringify!($($key).+), $next) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]))
  };

  // cases 3-6: no prefix

  // case 3: no prefix - own and pass to lazy_format
  // example: info!("hello world {}", some_display_struct);
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) $next:expr, $($rest:tt)*) => {
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , $next }} @ ($($prefix)* x) ($($past)* [$($prefix)*]) $($rest)*)
  };

  // case 4: `%` prefix - eagerly evaluate display string with `format!()`
  // example: info!("hello world {}", %display_struct);
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) %$next:expr, $($rest:tt)*) => {
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , format!("{}", $next) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]) $($rest)*)
  };

  // case 5: `?` prefix - eager evaluate debug string with `format!()`
  // example: info!("hello world {}", ?debug_struct);
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) ?$next:expr, $($rest:tt)*) => {
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , format!("{:?}", $next) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]) $($rest)*)
  };

  // case 6: `^` prefix - struct implements Serialize trait, encode into Store from buffer
  // example: info!("hello world {}", ^debug_struct);
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) ^$next:expr, $($rest:tt)*) => {
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , $crate::make_store!($next) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]) $($rest)*)
  };

  // recursive cases 3-6: no prefix

  // case 3
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) $next:expr) => {
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , $next }} @ ($($prefix)* x) ($($past)* [$($prefix)*]))
  };

  // case 4
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) %$next:expr) => {
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , format!("{}", $next) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]))
  };

  // case 5
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) ?$next:expr) => {
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , format!("{:?}", $next) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]))
  };

  // case 6
  ($lvl:expr, $static_str:literal @@ {{ $(,)* $($args:expr),* }} @ ($($prefix:tt)*) ($($past:tt)*) ^$next:expr) => {
    $crate::try_log!($lvl, $static_str @@ {{ $($args),* , $crate::make_store!($next) }} @ ($($prefix)* x) ($($past)* [$($prefix)*]))
  };
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

/// Trace level log
#[macro_export]
macro_rules! trace {
  {$static_str:literal} => ( $crate::log!($crate::level::Level::Trace, $static_str) );
  {$static_str:literal, $($args:tt)*} => ( $crate::log!($crate::level::Level::Trace, $static_str, $($args)*) );
}

/// Debug level log
#[macro_export]
macro_rules! debug {
  {$static_str:literal} => ( $crate::log!($crate::level::Level::Debug, $static_str) );
  {$static_str:literal, $($args:tt)*} => ( $crate::log!($crate::level::Level::Debug, $static_str, $($args)*) );
}

/// Info level log
#[macro_export]
macro_rules! info {
  {$static_str:literal} => ( $crate::log!($crate::level::Level::Info, $static_str) );
  {$static_str:literal, $($args:tt)*} => ( $crate::log!($crate::level::Level::Info, $static_str, $($args)*) );
}

/// Warn level log
#[macro_export]
macro_rules! warn {
  {$static_str:literal} => ( $crate::log!($crate::level::Level::Warn, $static_str) );
  {$static_str:literal, $($args:tt)*} => ( $crate::log!($crate::level::Level::Warn, $static_str, $($args)*) );
}

/// Error level log
#[macro_export]
macro_rules! error {
  {$static_str:literal} => ( $crate::log!($crate::level::Level::Error, $static_str) );
  {$static_str:literal, $($args:tt)*} => ( $crate::log!($crate::level::Level::Error, $static_str, $($args)*) );
}
