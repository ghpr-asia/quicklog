use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{parse_macro_input, parse_quote, Ident};

use crate::args::{replace_fields_expr, Args, PrefixedArg};
use crate::Level;

/// Parses token stream into the different components of `Args` and
/// generates required tokens from the inputs
pub(crate) fn expand(level: Level, input: TokenStream) -> TokenStream {
    expand_parsed(level, parse_macro_input!(input as Args)).into()
}

/// Main function for expanding the components parsed from the macro call
pub(crate) fn expand_parsed(level: Level, mut args: Args) -> TokenStream2 {
    let args_traits_check: Vec<_> = args
        .prefixed_fields
        .iter()
        .filter_map(|arg| match &arg.arg {
            PrefixedArg::Debug(a) => Some(quote! { debug_check(&#a); }),
            PrefixedArg::Display(a) => Some(quote! { display_check(&#a); }),
            PrefixedArg::Serialize(a) => Some(quote! { serialize_check(&#a); }),
            PrefixedArg::Normal(_) => None,
        })
        .collect();

    let (new_idents_declaration, fmt_arg_idents, prefixed_field_idents) =
        convert_args_to_idents(&args);

    let mut fmt_args = args.formatting_args;
    replace_fields_expr(
        &mut fmt_args,
        fmt_arg_idents.into_iter().map(|ident| parse_quote!(#ident)),
    );

    let fmt_str = args
        .format_string
        .take()
        .map(|s| s.value())
        .unwrap_or_else(String::new);
    // Insert extra spacing between format string and format fields for prefixed fields
    // if prefixed fields exist
    // e.g. info!(?debug_struct, "hello world {}", a) -> format!("hello world {} debug_struct={:?}", a,
    // debug_struct)
    let mut special_fmt_str = if fmt_str.is_empty() { "" } else { " " }.to_string();
    for field in args.prefixed_fields.iter() {
        special_fmt_str.push_str(field.formatter().as_str());
        special_fmt_str.push(' ');
    }
    let special_fmt_str = special_fmt_str.trim_end();

    quote! {{
        if quicklog::is_level_enabled!(#level) {
            use quicklog::{Log, make_container, serialize::Serialize};

            const fn debug_check<T: ::std::fmt::Debug + Clone>(_: &T) {}
            const fn display_check<T: ::std::fmt::Display + Clone>(_: &T) {}
            const fn serialize_check<T: Serialize>(_: &T) {}

            #(#args_traits_check)*

            #new_idents_declaration

            let log_record = quicklog::LogRecord {
                level: #level,
                module_path: module_path!(),
                file: file!(),
                line: line!(),
                log_line: make_container!(quicklog::lazy_format::make_lazy_format!(|f| {
                    write!(f, #fmt_str, #fmt_args)?;
                    write!(f, #special_fmt_str, #(#prefixed_field_idents),*)
                }))
            };

            quicklog::logger().log(log_record)
        } else {
            Ok(())
        }
        .unwrap_or(())
    }}
}

/// Generates new identifier tokens and their declarations for every special
/// and formatting argument
fn convert_args_to_idents(args: &Args) -> (TokenStream2, Vec<Ident>, Vec<Ident>) {
    let mut args_to_own: Vec<TokenStream2> = Vec::new();
    let mut arg_count = 0;

    let mut new_ident = || {
        arg_count += 1;
        Ident::new("x".repeat(arg_count).as_str(), Span::call_site())
    };

    let mut fmt_arg_idents = Vec::with_capacity(args.formatting_args.len());
    for fmt_arg in args.formatting_args.iter() {
        args_to_own.push(fmt_arg.arg.to_token_stream());
        fmt_arg_idents.push(new_ident());
    }

    let mut prefixed_field_idents = Vec::with_capacity(args.prefixed_fields.len());
    for field in args.prefixed_fields.iter() {
        match &field.arg {
            PrefixedArg::Serialize(i) => args_to_own.push(quote! {
                quicklog::make_store!(#i)
            }),
            _ => args_to_own.push(field.arg.to_token_stream()),
        }
        prefixed_field_idents.push(new_ident());
    }

    let new_idents = fmt_arg_idents.iter().chain(prefixed_field_idents.iter());

    // No need to declare anything if no format/special arguments passed
    if args_to_own.is_empty() {
        return (quote! {}, fmt_arg_idents, prefixed_field_idents);
    }

    (
        quote! {
            let (#(#new_idents),*) = (#( (#args_to_own).to_owned() ),*);
        },
        fmt_arg_idents,
        prefixed_field_idents,
    )
}
