use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::parse_macro_input;

use crate::args::{Args, PrefixedArg, PrefixedField};
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

/// Codegen for writing of prefixed fields to the buffer.
fn gen_write_field(prefixed_field: &PrefixedField) -> TokenStream2 {
    fn gen_write_arg<T: ToTokens>(arg: &PrefixedArg<T>) -> TokenStream2 {
        let formatter = arg.formatter();
        match arg {
            PrefixedArg::Debug(a) | PrefixedArg::Display(a) => {
                quote! { cursor.write_fmt(fmt_buffer, format_args!(#formatter, &#a))?; }
            }
            PrefixedArg::Serialize(a) => {
                quote! { cursor.write_serialize(&#a)?; }
            }
        }
    }

    match prefixed_field {
        PrefixedField::Unnamed(ident) => gen_write_arg(ident),
        PrefixedField::Named(field) => gen_write_arg(&field.arg),
    }
}

/// Parses token stream into the different components of `Args` and
/// generates required tokens from the inputs
pub(crate) fn expand(level: Level, input: TokenStream) -> TokenStream {
    expand_parsed(level, parse_macro_input!(input as Args)).into()
}

/// Main function for expanding the components parsed from the macro call
pub(crate) fn expand_parsed(level: Level, args: Args) -> TokenStream2 {
    let prefixed_args_write: Vec<_> = args.prefixed_fields.iter().map(gen_write_field).collect();

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

            static META: quicklog::queue::Metadata = quicklog::queue::Metadata {
                module_path: module_path!(),
                file: file!(),
                line: line!(),
                level: #level,
                format_str: #fmt_str
            };

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
