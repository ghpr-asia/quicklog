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
    expand(Level::Trace, input)
}

/// *Debug* level log.
#[proc_macro]
pub fn debug(input: TokenStream) -> TokenStream {
    expand(Level::Debug, input)
}

/// *Info* level log.
#[proc_macro]
pub fn info(input: TokenStream) -> TokenStream {
    expand(Level::Info, input)
}

/// *Warn* level log.
#[proc_macro]
pub fn warn(input: TokenStream) -> TokenStream {
    expand(Level::Warn, input)
}

/// *Error* level log.
#[proc_macro]
pub fn error(input: TokenStream) -> TokenStream {
    expand(Level::Error, input)
}

/// Derive macro for generating `Serialize` implementations.
#[proc_macro_derive(Serialize)]
pub fn derive_serialize(input: TokenStream) -> TokenStream {
    derive(input)
}
