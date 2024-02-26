use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};

// TODO: Duplicate of quicklog's Level, re-defined here to avoid including quicklog
// as a dependency
// Alternative is to break out a subset of quicklog into a separate `quicklog-core`
// crate or something similar, but that can be done in a future more general
// refactor.
pub(crate) enum Level {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

impl ToTokens for Level {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let tok = match self {
            Self::Trace => quote! { quicklog::level::Level::Trace },
            Self::Debug => quote! { quicklog::level::Level::Debug },
            Self::Info => quote! { quicklog::level::Level::Info },
            Self::Warn => quote! { quicklog::level::Level::Warn },
            Self::Error => quote! { quicklog::level::Level::Error },
        };

        tokens.extend(tok);
    }
}
