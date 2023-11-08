use proc_macro2::{TokenStream as TokenStream2, TokenTree};
use quote::{quote, ToTokens};
use syn::{
    parse::{self, Parse, ParseStream},
    punctuated::Punctuated,
    Expr, Ident, LitStr, Token,
};

use crate::format_arg::FormatArg;

/// Dot-delimited identifiers, e.g. `ident_a.some_field.other_field`
pub(crate) type DotDelimitedIdent = Punctuated<Ident, Token![.]>;

/// Comma-separated sequence of `PrefixedArg`-based named fields
/// e.g. `my.name = ?debug_struct`, `%display_struct`
pub(crate) type PrefixedFields = Punctuated<NamedField<PrefixedArg>, Token![,]>;

/// Comma-separated sequence of `Expr`-based named fields
/// e.g. `my.name = debug_struct`, `display_struct`
/// Similar to `PrefixedFields`, but doesn't allow for prefixes for the
/// main field argument, since those are not valid Rust expressions
pub(crate) type ExprFields = Punctuated<NamedField<Expr>, Token![,]>;

/// Formatting argument with an optional prefix
/// e.g. `?debug_struct`, `%display_struct`, `serialize_struct`
#[derive(Clone)]
pub(crate) enum PrefixedArg {
    /// `?debug_struct`
    Debug(Expr),
    /// `%display_struct`
    Display(Expr),
    /// `serialize_struct` (no prefix by default)
    Serialize(Expr),
}

impl PrefixedArg {
    /// The captured expression for this argument
    pub(crate) fn expr(&self) -> &Expr {
        match self {
            Self::Debug(i) | Self::Display(i) | Self::Serialize(i) => i,
        }
    }
}

impl ToTokens for PrefixedArg {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.expr().to_tokens(tokens);
    }
}

impl Parse for PrefixedArg {
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

impl FormatArg for PrefixedArg {
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

impl<T: Parse + FormatArg + ToTokens> NamedField<T> {
    /// Returns identifier tokens for the name describing this field
    /// If the original form was parsed from something like "my_name = foo",
    /// then identifier is `my_name`. Otherwise, the name is taken from the
    /// expression, which would just be `foo` in this case.
    pub(crate) fn name(&self) -> TokenStream2 {
        if let Some(n) = &self.name {
            n.into_token_stream()
        } else {
            (&self.arg).into_token_stream()
        }
    }
}

impl<T: Parse + ToTokens> Parse for NamedField<T> {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        // Look ahead to check if this contains an assignment
        let begin = input.fork();
        let mut cursor = begin.cursor();
        let mut has_assign = false;
        while let Some((tt, next)) = cursor.token_tree() {
            match &tt {
                TokenTree::Punct(punct) if punct.as_char() == ',' => break,
                TokenTree::Punct(punct) if punct.as_char() == '=' => {
                    has_assign = true;
                    break;
                }
                _ => cursor = next,
            }
        }

        let (name, assign) = if has_assign {
            (
                Some(DotDelimitedIdent::parse_separated_nonempty(input)?),
                Some(input.parse()?),
            )
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
