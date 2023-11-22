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
#[proc_macro]
pub fn trace(input: TokenStream) -> TokenStream {
    expand(Level::Trace, input, false)
}

/// *Trace* level log, with deferred committing.
#[proc_macro]
pub fn trace_defer(input: TokenStream) -> TokenStream {
    expand(Level::Trace, input, true)
}

/// *Debug* level log.
#[proc_macro]
pub fn debug(input: TokenStream) -> TokenStream {
    expand(Level::Debug, input, false)
}

/// *Debug* level log, with deferred committing.
#[proc_macro]
pub fn debug_defer(input: TokenStream) -> TokenStream {
    expand(Level::Debug, input, true)
}

/// *Info* level log.
#[proc_macro]
pub fn info(input: TokenStream) -> TokenStream {
    expand(Level::Info, input, false)
}

/// *Info* level log, with deferred committing.
#[proc_macro]
pub fn info_defer(input: TokenStream) -> TokenStream {
    expand(Level::Info, input, true)
}

/// *Warn* level log.
#[proc_macro]
pub fn warn(input: TokenStream) -> TokenStream {
    expand(Level::Warn, input, false)
}

/// *Warn* level log, with deferred committing.
#[proc_macro]
pub fn warn_defer(input: TokenStream) -> TokenStream {
    expand(Level::Warn, input, true)
}

/// *Error* level log.
#[proc_macro]
pub fn error(input: TokenStream) -> TokenStream {
    expand(Level::Error, input, false)
}

/// *Error* level log, with deferred committing.
#[proc_macro]
pub fn error_defer(input: TokenStream) -> TokenStream {
    expand(Level::Error, input, true)
}

/// Derive macro for generating `Serialize` implementations.
#[proc_macro_derive(Serialize)]
pub fn derive_serialize(input: TokenStream) -> TokenStream {
    derive(input)
}
