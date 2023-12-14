use std::str::FromStr;

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};

pub(crate) enum ParseError {
    VariantNotFound,
}

// TODO: Duplicate of quicklog's Level, re-defined here to avoid including
// quicklog as a dependency
// Alternative is to break out a subset of quicklog into a separate
// `quicklog-core` crate or something similar, but that can be done in a future
// more general refactor.
#[derive(Copy, Clone, PartialOrd, PartialEq, Eq)]
pub(crate) enum Level {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Event = 5,
}

impl FromStr for Level {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trc" | "trace" | "0" => Ok(Level::Trace),
            "dbg" | "debug" | "1" => Ok(Level::Debug),
            "inf" | "info" | "2" => Ok(Level::Info),
            "wrn" | "warn" | "3" => Ok(Level::Warn),
            "err" | "error" | "4" => Ok(Level::Error),
            "evt" | "event" | "5" => Ok(Level::Event),
            _ => Err(ParseError::VariantNotFound),
        }
    }
}

impl ToTokens for Level {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let tok = match self {
            Self::Trace => quote! { quicklog::level::Level::Trace },
            Self::Debug => quote! { quicklog::level::Level::Debug },
            Self::Info => quote! { quicklog::level::Level::Info },
            Self::Warn => quote! { quicklog::level::Level::Warn },
            Self::Error => quote! { quicklog::level::Level::Error },
            Self::Event => quote! { quicklog::level::Level::Event },
        };

        tokens.extend(tok);
    }
}
