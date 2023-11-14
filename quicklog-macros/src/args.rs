use proc_macro2::{TokenStream as TokenStream2, TokenTree};
use quote::{quote, ToTokens};
use syn::{
    parse::{self, Parse, ParseStream},
    punctuated::Punctuated,
    Expr, Ident, LitStr, Token,
};

use crate::format_arg::FormatArg;

/// Dot-delimited identifiers with optional prefix;
pub(crate) type PrefixedIdent = PrefixedArg<DotDelimitedIdent>;

/// Expressions with optional prefix.
pub(crate) type PrefixedExpr = PrefixedArg<Expr>;

/// Comma-separated sequence of `PrefixedArg`-based named fields
/// e.g. `my.name = ?debug_struct`, `%display_struct`
pub(crate) type PrefixedFields = Punctuated<PrefixedField, Token![,]>;

/// Comma-separated sequence of `Expr`-based named fields
/// e.g. `my.name = debug_struct`, `display_struct`
/// Similar to `PrefixedFields`, but doesn't allow for prefixes for the
/// main field argument, since those are not valid Rust expressions
pub(crate) type ExprFields = Punctuated<NamedField<Expr>, Token![,]>;

/// Dot-delimited identifiers, e.g. `ident_a.some_field.other_field`
pub(crate) struct DotDelimitedIdent(Punctuated<Ident, Token![.]>);

impl Parse for DotDelimitedIdent {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(Punctuated::parse_separated_nonempty(input)?))
    }
}

impl ToTokens for DotDelimitedIdent {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.0.to_tokens(tokens);
    }
}

/// Optionally-named prefixed fields.
pub(crate) enum PrefixedField {
    /// If unnamed, then can only be an optionally-prefixed dot-delimited ident.
    /// e.g. `my.struct.field`, `?my.debug_struct.inner`
    Unnamed(PrefixedIdent),
    /// If named, then can be an optionally-prefixed expression.
    /// e.g. `?(5 + 1)`, `%"hello world"`, `some_struct.inner`
    Named(NamedField<PrefixedExpr>),
}

impl PrefixedField {
    pub(crate) fn name(&self) -> TokenStream2 {
        match self {
            Self::Unnamed(ident) => ident.name(),
            Self::Named(f) => {
                // During parsing, guaranteed that `PrefixedField` will have a
                // name
                f.name.as_ref().unwrap().into_token_stream()
            }
        }
    }

    pub(crate) fn arg(&self) -> TokenStream2 {
        match self {
            Self::Unnamed(ident) => ident.name(),
            Self::Named(f) => f.arg.expr().into_token_stream(),
        }
    }

    pub(crate) fn is_serialize(&self) -> bool {
        matches!(
            self,
            Self::Unnamed(PrefixedArg::Serialize(_))
                | Self::Named(NamedField {
                    arg: PrefixedArg::Serialize(_),
                    ..
                })
        )
    }
}

impl Parse for PrefixedField {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Look ahead to check if this contains a name
        let begin = input.fork();
        let mut cursor = begin.cursor();
        let mut has_name = false;
        while let Some((tt, next)) = cursor.token_tree() {
            match &tt {
                TokenTree::Punct(punct) if punct.as_char() == ',' => break,
                TokenTree::Punct(punct) if punct.as_char() == '=' => {
                    has_name = true;
                    break;
                }
                _ => cursor = next,
            }
        }

        let parsed = if has_name {
            Self::Named(input.parse()?)
        } else {
            Self::Unnamed(input.parse()?)
        };

        Ok(parsed)
    }
}

/// Formatting argument with an optional prefix.
/// e.g. `?debug_struct`, `%display_struct`, `serialize_struct`
#[derive(Clone)]
pub(crate) enum PrefixedArg<T> {
    /// `?debug_struct`
    Debug(T),
    /// `%display_struct`
    Display(T),
    /// `serialize_struct` (no prefix by default)
    Serialize(T),
}

impl PrefixedIdent {
    /// The underlying identifier
    fn name(&self) -> TokenStream2 {
        match self {
            PrefixedArg::Debug(i) | PrefixedArg::Display(i) | PrefixedArg::Serialize(i) => {
                i.into_token_stream()
            }
        }
    }
}

impl PrefixedExpr {
    /// The captured expression for this argument
    pub(crate) fn expr(&self) -> &Expr {
        match self {
            Self::Debug(i) | Self::Display(i) | Self::Serialize(i) => i,
        }
    }
}

impl ToTokens for PrefixedExpr {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.expr().to_tokens(tokens);
    }
}

impl<T: Parse> Parse for PrefixedArg<T> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![?]) {
            input.parse::<Token![?]>()?;

            Ok(PrefixedArg::Debug(input.parse()?))
        } else if input.peek(Token![%]) {
            input.parse::<Token![%]>()?;

            Ok(PrefixedArg::Display(input.parse()?))
        } else {
            Ok(PrefixedArg::Serialize(input.parse()?))
        }
    }
}

impl<T> FormatArg for PrefixedArg<T> {
    fn formatter(&self) -> &'static str {
        match self {
            Self::Debug(_) => "{:?}",
            Self::Display(_) | Self::Serialize(_) => "{}",
        }
    }
}

/// Describes a logging argument of the form `a.b.c = ?debug_struct`, `a.b.c =
/// some_expr()`
pub(crate) struct NamedField<T: Parse> {
    /// `a.b.c`, optional
    pub(crate) name: Option<DotDelimitedIdent>,
    /// `=` token, optional
    pub(crate) assign: Option<Token![=]>,
    /// `?debug_struct`, `some_expr()`
    pub(crate) arg: T,
}

impl<T: Parse + ToTokens> Parse for NamedField<T> {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        // Look ahead to check if this contains a name
        // NOTE: we need to perform this check because `T` can be an expr, and
        // `a = some_expr()` *is* an expression of its own!
        // So directly doing `T::parse(input)` can consume the `=` token,
        // which means we lose information about the name.
        let begin = input.fork();
        let mut cursor = begin.cursor();
        let mut has_name = false;
        while let Some((tt, next)) = cursor.token_tree() {
            match &tt {
                TokenTree::Punct(punct) if punct.as_char() == ',' => break,
                TokenTree::Punct(punct) if punct.as_char() == '=' => {
                    has_name = true;
                    break;
                }
                _ => cursor = next,
            }
        }

        let (name, assign) = if has_name {
            (Some(input.parse()?), Some(input.parse()?))
        } else {
            (None, None)
        };

        Ok(Self {
            name,
            assign,
            arg: input.parse()?,
        })
    }
}

impl<T: Parse + ToTokens> ToTokens for NamedField<T> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let arg = &self.arg;
        let output = self
            .name
            .as_ref()
            .zip(self.assign.as_ref())
            .map(|(name, assign_token)| {
                quote! { #name #assign_token #arg }
            })
            .unwrap_or_else(|| {
                quote! { #arg }
            });

        tokens.extend(output);
    }
}

/// Contains the different components of a logging command.
/// Consider an example macro call:
/// ```ignore
/// info!(a = ?debug_struct, %display_struct, "Hello World {some_data}", some_data = "me!") ;
///       ----------------------------------  -------------------------  -----------------
///       |                                   |                          |
///       |                                   |                          |
///       Prefixed field(s)                   Format string              Format argument(s)
/// ```
/// We split arguments passed to the macro call into 3 components. They are:
/// 1. Prefixed fields
///   - These are the (optionally) prefixed variables that will be specially
///     appended to the end of the format string.
/// 2. Format string
///   - The format string, the same as that used in `format!`
/// 3. Format arguments
///   - These are the expressions that will be substituted into the format
///     string, similar to how `format!` works.
///
/// Having these separate components in mind can be useful for understanding
/// how the logging macros expand out.
pub(crate) struct Args {
    /// `?debug_struct`, `%display_struct`
    pub(crate) prefixed_fields: PrefixedFields,
    /// `"Hello World {some_data}"`
    pub(crate) format_string: Option<LitStr>,
    /// `some_data = "me!"`
    pub(crate) formatting_args: ExprFields,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        if input.is_empty() {
            return Err(input.error("no tokens passed to macro"));
        }

        let mut prefixed_fields: PrefixedFields = Punctuated::new();
        loop {
            if input.is_empty() || input.peek(LitStr) {
                // No more prefixed fields
                // Or encountered format string, so no longer accepting prefixed
                // fields
                break;
            }

            prefixed_fields.push_value(input.parse()?);
            if let Some(comma) = input.parse::<Option<Token![,]>>()? {
                prefixed_fields.push_punct(comma);
            } else {
                break;
            }
        }

        if let Ok(format_string) = input.parse::<LitStr>() {
            // Start parsing formatting args, if any
            let formatting_args = if !input.is_empty() {
                input.parse::<Token![,]>()?;

                Punctuated::parse_separated_nonempty(input)?
            } else {
                ExprFields::new()
            };

            Ok(Self {
                prefixed_fields,
                format_string: Some(format_string),
                formatting_args,
            })
        } else {
            // No format string, just terminate
            Ok(Self {
                prefixed_fields,
                format_string: None,
                formatting_args: ExprFields::new(),
            })
        }
    }
}
