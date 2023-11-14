use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::parse_macro_input;

use crate::args::{Args, PrefixedArg, PrefixedField};
use crate::format_arg::FormatArg;
use crate::Level;

struct Codegen {
    fmt_args: TokenStream2,
    metadata: TokenStream2,
    prefixed_args: TokenStream2,
}

impl Codegen {
    fn new(args: &Args, level: &Level) -> Self {
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
        let (args_kind, fmt_args_write, prefixed_args_write, mut fmt_str) = if need_write_fmt_str {
            let fmt_args = if has_fmt_args {
                let args = &args.formatting_args;
                quote! { , #args }
            } else {
                quote! {}
            };

            // Entire format string + format args are wrapped into one argument
            let num_args = args.prefixed_fields.len() + 1;
            (
                quote! { quicklog::queue::ArgsKind::Normal(#num_args) },
                quote! { cursor.write_fmt(fmt_buffer, format_args!(#original_fmt_str #fmt_args))?; },
                args.prefixed_fields.iter().map(gen_write_field).collect(),
                "{}".to_string(),
            )
        } else {
            let all_serialize = !args.prefixed_fields.is_empty()
                && args.prefixed_fields.iter().all(PrefixedField::is_serialize);
            let (args_kind, prefixed_args_write) = if all_serialize {
                // Special optimization if all arguments use `Serialize` impl and
                // no need to write format string
                let args: Vec<_> = args.prefixed_fields.iter().map(|f| f.arg()).collect();

                (
                    quote! { quicklog::queue::ArgsKind::AllSerialize },
                    quote! {
                        cursor.write_serialize_tpl(&(#(&#args,)*))?;
                    },
                )
            } else {
                let num_args = args.prefixed_fields.len();
                (
                    quote! { quicklog::queue::ArgsKind::Normal(#num_args) },
                    args.prefixed_fields.iter().map(gen_write_field).collect(),
                )
            };

            (args_kind, quote! {}, prefixed_args_write, original_fmt_str)
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

        let metadata_write = quote! {
            static META: quicklog::queue::Metadata = quicklog::queue::Metadata::new(
                std::module_path!(),
                std::file!(),
                std::line!(),
                #level,
                #fmt_str,
                #args_kind,
            );
        };

        Self {
            fmt_args: fmt_args_write,
            metadata: metadata_write,
            prefixed_args: prefixed_args_write,
        }
    }
}

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
    let Codegen {
        prefixed_args,
        fmt_args,
        metadata,
    } = Codegen::new(&args, &level);

    quote! {{
        if quicklog::is_level_enabled!(#level) {
            use quicklog::{serialize::Serialize};

            #metadata

            (|| {
                let mut logger = quicklog::logger();
                let now = logger.now();
                let (mut chunk, fmt_buffer) = logger.prepare_write();
                let (first, second) = chunk.as_mut_slices();
                let mut cursor = quicklog::queue::CursorMut::new(first, second);

                let header = quicklog::queue::LogHeader::new(&META, now);
                cursor.write(&header)?;

                #fmt_args

                #prefixed_args

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
