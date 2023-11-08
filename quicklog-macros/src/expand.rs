use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse_macro_input;

use crate::args::{Args, PrefixedArg};
use crate::format_arg::FormatArg;
use crate::Level;

/// Checks if the passed format string has any format specifiers, e.g. {}, {:?},
/// {a}.
fn has_fmt_specifiers(fmt_str: &str) -> bool {
    let mut chars = fmt_str.chars();
    while let Some(c) = chars.next() {
        if c != '{' {
            continue;
        }

        if chars.as_str().starts_with('{') {
            // Escaped '{{'
            chars.next();
            continue;
        }

        // Might have unmatched open bracket, so explicitly check for presence
        // of close bracket
        return chars.as_str().find('}').is_some();
    }

    false
}

/// Parses token stream into the different components of `Args` and
/// generates required tokens from the inputs
pub(crate) fn expand(level: Level, input: TokenStream) -> TokenStream {
    expand_parsed(level, parse_macro_input!(input as Args)).into()
}

/// Main function for expanding the components parsed from the macro call
pub(crate) fn expand_parsed(level: Level, args: Args) -> TokenStream2 {
    let (args_traits_check, prefixed_args_write) = {
        let mut args_traits_check = Vec::new();
        let mut args_write = Vec::new();

        for prefixed_field in &args.prefixed_fields {
            let formatter = prefixed_field.arg.formatter();
            match &prefixed_field.arg {
                PrefixedArg::Serialize(a) => {
                    args_traits_check.push(quote! { serialize_check(&#a); });
                    args_write.push(quote! { cursor.write_serialize(&#a)?; });
                }
                PrefixedArg::Debug(a) | PrefixedArg::Display(a) | PrefixedArg::Normal(a) => {
                    args_write.push(
                        quote! { cursor.write_fmt(fmt_buffer, format_args!(#formatter, &#a))?; },
                    );
                }
            }
        }

        (args_traits_check, args_write)
    };

    let original_fmt_str = args
        .format_string
        .as_ref()
        .map(|s| s.value())
        .unwrap_or_else(String::new);
    let has_fmt_args = !args.formatting_args.is_empty();
    let need_write_fmt_str = has_fmt_args || has_fmt_specifiers(original_fmt_str.as_str());

    // If there are format arguments or the format string has format specifiers
    // (might be named captures), then we need to format it. Otherwise, we
    // can just pass the format string along with `Metadata` and avoid one
    // formatting operation.
    let (num_args, fmt_args_write, mut fmt_str) = if need_write_fmt_str {
        let fmt_args = if has_fmt_args {
            let args = &args.formatting_args;
            quote! { , #args }
        } else {
            quote! {}
        };

        // Entire format string + format args are wrapped into one argument
        (
            args.prefixed_fields.len() + 1,
            quote! { cursor.write_fmt(fmt_buffer, format_args!(#original_fmt_str #fmt_args))?; },
            "{}".to_string(),
        )
    } else {
        (args.prefixed_fields.len(), quote! {}, original_fmt_str)
    };

    // Construct format string for prefixed fields and append to original
    // format string
    if !fmt_str.is_empty() && !args.prefixed_fields.is_empty() {
        fmt_str.push(' ');
    }
    let num_prefixed_fields = args.prefixed_fields.len();
    for (idx, field) in args.prefixed_fields.iter().enumerate() {
        let fmt_name = field.name().to_string() + "={}";
        fmt_str.push_str(fmt_name.as_str());

        if idx < num_prefixed_fields - 1 {
            fmt_str.push(' ');
        }
    }

    quote! {{
        if quicklog::is_level_enabled!(#level) {
            use quicklog::{serialize::Serialize};

            const fn serialize_check<T: Serialize>(_: &T) {}

            static META: quicklog::queue::Metadata = quicklog::queue::Metadata {
                module_path: module_path!(),
                file: file!(),
                line: line!(),
                level: #level,
                format_str: #fmt_str
            };

            #(#args_traits_check)*

            (|| {
                let mut logger = quicklog::logger();
                let now = logger.now();
                let (mut chunk, fmt_buffer) = logger.prepare_write();
                let (first, second) = chunk.as_mut_slices();
                let mut cursor = quicklog::queue::CursorMut::new(first, second);

                let header = quicklog::queue::LogHeader {
                    metadata: &META,
                    instant: now,
                    num_args: #num_args,
                };
                cursor.write(&header)?;

                #fmt_args_write

                #(#prefixed_args_write)*

                let commit_size = cursor.finish();
                quicklog::Quicklog::finish_write(chunk, commit_size);

                Ok::<(), quicklog::queue::WriteError>(())
            })()
        } else {
            Ok(())
        }
        .unwrap_or(())
    }}
}