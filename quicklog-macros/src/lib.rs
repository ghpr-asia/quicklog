use proc_macro::TokenStream;

mod args;
mod derive;
mod expand;
mod format_arg;
mod quicklog;

use derive::derive;
use expand::expand;
use quicklog::Level;

/// *Trace* level log.
///
/// # Examples
///
/// ```rust ignore
/// use quicklog::{flush, init, trace, Serialize};
///
/// #[derive(Serialize)]
/// struct UserStruct;
///
/// # fn main() {
/// init!();
/// let (msg, payload) = ("Hello from the other side", vec![1, 2, 3]);
/// let data = UserStruct;
///
/// trace!(data, src = ?payload, "Message: {}", msg);
/// trace!("Data: {data:^}");
///
/// assert!(flush!().is_ok());
/// # }
/// ```
#[proc_macro]
pub fn trace(input: TokenStream) -> TokenStream {
    expand(Level::Trace, input, false)
}

/// *Trace* level log, with deferred committing.
///
/// Has to be followed by a call to `commit!` or a non-`defer` logging macro
/// for this log to be available. See also the `commit_on_scope_end` macro for
/// guaranteeing that commits run at the end of the current scope, even if it
/// exits early.
///
/// # Examples
///
/// ```rust ignore
/// use quicklog::{commit, flush, init, trace_defer, Serialize};
///
/// #[derive(Serialize)]
/// struct UserStruct;
///
/// # fn main() {
/// init!();
/// let (msg, payload) = ("Hello from the other side", vec![1, 2, 3]);
/// let data = UserStruct;
///
/// trace_defer!(data, src = ?payload, "Message: {}", msg);
/// trace_defer!("Data: {data:^}");
///
/// commit!();
/// while let Ok(()) = flush!() {}
/// # }
/// ```
#[proc_macro]
pub fn trace_defer(input: TokenStream) -> TokenStream {
    expand(Level::Trace, input, true)
}

/// *Debug* level log.
///
/// # Examples
///
/// ```rust ignore
/// use quicklog::{debug, flush, init, Serialize};
///
/// #[derive(Serialize)]
/// struct UserStruct;
///
/// # fn main() {
/// init!();
/// let (msg, payload) = ("Hello from the other side", vec![1, 2, 3]);
/// let data = UserStruct;
///
/// debug!(data, src = ?payload, "Message: {}", msg);
/// debug!("Data: {data:^}");
///
/// assert!(flush!().is_ok());
/// # }
/// ```
#[proc_macro]
pub fn debug(input: TokenStream) -> TokenStream {
    expand(Level::Debug, input, false)
}

/// *Debug* level log, with deferred committing.
///
/// Has to be followed by a call to `commit!` or a non-`defer` logging macro
/// for this log to be available. See also the `commit_on_scope_end` macro for
/// guaranteeing that commits run at the end of the current scope, even if it
/// exits early.
///
/// # Examples
///
/// ```rust ignore
/// use quicklog::{commit, debug_defer, flush, init, Serialize};
///
/// #[derive(Serialize)]
/// struct UserStruct;
///
/// # fn main() {
/// init!();
/// let (msg, payload) = ("Hello from the other side", vec![1, 2, 3]);
/// let data = UserStruct;
///
/// debug_defer!(data, src = ?payload, "Message: {}", msg);
/// debug_defer!("Data: {data:^}");
///
/// commit!();
/// while let Ok(()) = flush!() {}
/// # }
/// ```
#[proc_macro]
pub fn debug_defer(input: TokenStream) -> TokenStream {
    expand(Level::Debug, input, true)
}

/// *Info* level log.
///
/// # Examples
///
/// ```rust ignore
/// use quicklog::{flush, info, init, Serialize};
///
/// #[derive(Serialize)]
/// struct UserStruct;
///
/// # fn main() {
/// init!();
/// let (msg, payload) = ("Hello from the other side", vec![1, 2, 3]);
/// let data = UserStruct;
///
/// info!(data, src = ?payload, "Message: {}", msg);
/// info!("Data: {data:^}");
///
/// assert!(flush!().is_ok());
/// # }
/// ```
#[proc_macro]
pub fn info(input: TokenStream) -> TokenStream {
    expand(Level::Info, input, false)
}

/// *Info* level log, with deferred committing.
///
/// Has to be followed by a call to `commit!` or a non-`defer` logging macro
/// for this log to be available. See also the `commit_on_scope_end` macro for
/// guaranteeing that commits run at the end of the current scope, even if it
/// exits early.
///
/// # Examples
///
/// ```rust ignore
/// use quicklog::{commit, flush, info_defer, init, Serialize};
///
/// #[derive(Serialize)]
/// struct UserStruct;
///
/// # fn main() {
/// init!();
/// let (msg, payload) = ("Hello from the other side", vec![1, 2, 3]);
/// let data = UserStruct;
///
/// info_defer!(data, src = ?payload, "Message: {}", msg);
/// info_defer!("Data: {data:^}");
///
/// commit!();
/// while let Ok(()) = flush!() {}
/// # }
/// ```
#[proc_macro]
pub fn info_defer(input: TokenStream) -> TokenStream {
    expand(Level::Info, input, true)
}

/// *Warn* level log.
///
/// # Examples
///
/// ```rust ignore
/// use quicklog::{flush, init, warn, Serialize};
///
/// #[derive(Serialize)]
/// struct UserStruct;
///
/// # fn main() {
/// init!();
/// let (msg, payload) = ("Hello from the other side", vec![1, 2, 3]);
/// let data = UserStruct;
///
/// warn!(data, src = ?payload, "Message: {}", msg);
/// warn!("Data: {data:^}");
///
/// assert!(flush!().is_ok());
/// # }
/// ```
#[proc_macro]
pub fn warn(input: TokenStream) -> TokenStream {
    expand(Level::Warn, input, false)
}

/// *Warn* level log, with deferred committing.
///
/// Has to be followed by a call to `commit!` or a non-`defer` logging macro
/// for this log to be available. See also the `commit_on_scope_end` macro for
/// guaranteeing that commits run at the end of the current scope, even if it
/// exits early.
///
/// # Examples
///
/// ```rust ignore
/// use quicklog::{commit, flush, init, warn_defer, Serialize};
///
/// #[derive(Serialize)]
/// struct UserStruct;
///
/// # fn main() {
/// init!();
/// let (msg, payload) = ("Hello from the other side", vec![1, 2, 3]);
/// let data = UserStruct;
///
/// warn_defer!(data, src = ?payload, "Message: {}", msg);
/// warn_defer!("Data: {data:^}");
///
/// commit!();
/// while let Ok(()) = flush!() {}
/// # }
/// ```
#[proc_macro]
pub fn warn_defer(input: TokenStream) -> TokenStream {
    expand(Level::Warn, input, true)
}

/// *Error* level log.
///
/// # Examples
///
/// ```rust ignore
/// use quicklog::{error, flush, init, Serialize};
///
/// #[derive(Serialize)]
/// struct UserStruct;
///
/// # fn main() {
/// init!();
/// let (msg, payload) = ("Hello from the other side", vec![1, 2, 3]);
/// let data = UserStruct;
///
/// error!(data, src = ?payload, "Message: {}", msg);
/// error!("Data: {data:^}");
///
/// assert!(flush!().is_ok());
/// # }
/// ```
#[proc_macro]
pub fn error(input: TokenStream) -> TokenStream {
    expand(Level::Error, input, false)
}

/// *Error* level log, with deferred committing.
///
/// Has to be followed by a call to `commit!` or a non-`defer` logging macro
/// for this log to be available. See also the `commit_on_scope_end` macro for
/// guaranteeing that commits run at the end of the current scope, even if it
/// exits early.
///
/// # Examples
///
/// ```rust ignore
/// use quicklog::{commit, error_defer, flush, init, Serialize};
///
/// #[derive(Serialize)]
/// struct UserStruct;
///
/// # fn main() {
/// init!();
/// let (msg, payload) = ("Hello from the other side", vec![1, 2, 3]);
/// let data = UserStruct;
///
/// error_defer!(data, src = ?payload, "Message: {}", msg);
/// error_defer!("Data: {data:^}");
///
/// commit!();
/// while let Ok(()) = flush!() {}
/// # }
/// ```
#[proc_macro]
pub fn error_defer(input: TokenStream) -> TokenStream {
    expand(Level::Error, input, true)
}

/// Special macro for JSON formatting.
///
/// # Examples
///
/// ```rust ignore
/// use quicklog::{event, flush, init, Serialize};
///
/// #[derive(Serialize)]
/// struct UserStruct;
///
/// # fn main() {
/// init!();
/// let (msg, payload) = ("Hello from the other side", vec![1, 2, 3]);
/// let data = UserStruct;
///
/// event!(data, src = ?payload, "Message: {}", msg);
/// event!("Data: {data:^}");
///
/// assert!(flush!().is_ok());
/// # }
/// ```
#[proc_macro]
pub fn event(input: TokenStream) -> TokenStream {
    expand(Level::Event, input, false)
}

/// Special macro for JSON formatting, with deferred committing.
///
/// Has to be followed by a call to `commit!` or a non-`defer` logging macro
/// for this log to be available. See also the `commit_on_scope_end` macro for
/// guaranteeing that commits run at the end of the current scope, even if it
/// exits early.
///
/// # Examples
///
/// ```rust ignore
/// use quicklog::{commit, event_defer, flush, init, Serialize};
///
/// #[derive(Serialize)]
/// struct UserStruct;
///
/// # fn main() {
/// init!();
/// let (msg, payload) = ("Hello from the other side", vec![1, 2, 3]);
/// let data = UserStruct;
///
/// event_defer!(data, src = ?payload, "Message: {}", msg);
/// event_defer!("Data: {data:^}");
///
/// commit!();
/// while let Ok(()) = flush!() {}
/// # }
/// ```
#[proc_macro]
pub fn event_defer(input: TokenStream) -> TokenStream {
    expand(Level::Event, input, true)
}

/// Derive macro for generating `Serialize` implementations.
///
/// Intended to be use in a similar fashion as [`Debug`](std::fmt::Debug).
///
/// # Examples
///
/// ```rust ignore
/// use quicklog::{flush, info, init, Serialize};
///
/// #[derive(Debug, Serialize)]
/// struct UserStruct {
///     a: usize,
///     b: i32,
///     c: String,
///     d: Vec<u8>,
/// }
///
/// #[derive(Serialize)]
/// enum TestEnum {
///     Foo,
///     Bar { t: usize, },
///     Baz { msg: String, arr: [usize; 2] },
/// }
///
/// # fn main() {
/// init!();
/// let user_struct = UserStruct {
///     a: 999,
///     b: -420,
///     c: "Hello world".to_string(),
///     d: vec![65, 67, 68, 70],
/// };
/// let test_enum = TestEnum::Baz { msg: "Hello world 2".to_string(), arr: [0, 2] };
///
/// info!(user_struct, user_enum = test_enum, debug_struct = ?user_struct, "Hello from the other side: {:^}", "some receiver");
/// assert!(flush!().is_ok());
/// # }
/// ```
#[proc_macro_derive(Serialize)]
pub fn derive_serialize(input: TokenStream) -> TokenStream {
    derive(input)
}
