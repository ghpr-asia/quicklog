use proc_macro::TokenStream;

mod args;
mod expand;
mod format_arg;
mod quicklog;

use expand::expand;
use quicklog::Level;

#[proc_macro]
pub fn trace(input: TokenStream) -> TokenStream {
    expand(Level::Trace, input)
}

#[proc_macro]
pub fn debug(input: TokenStream) -> TokenStream {
    expand(Level::Debug, input)
}

#[proc_macro]
pub fn info(input: TokenStream) -> TokenStream {
    expand(Level::Info, input)
}

#[proc_macro]
pub fn warn(input: TokenStream) -> TokenStream {
    expand(Level::Warn, input)
}

#[proc_macro]
pub fn error(input: TokenStream) -> TokenStream {
    expand(Level::Error, input)
}
