use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use syn::parse_macro_input;

use crate::args::{Args, PrefixedArg, PrefixedField};
use crate::Level;

struct IdentGen(Vec<Ident>);

impl IdentGen {
    const fn new() -> Self {
        Self(Vec::new())
    }

    fn gen(&mut self) -> &Ident {
        let idx = self.0.len();
        let ident = Ident::new(&"x".repeat(idx + 1), Span::call_site());
        self.0.push(ident);

        self.0.last().unwrap()
    }
}

struct Codegen {
    prologue: TokenStream2,
    fmt_args: TokenStream2,
    metadata: TokenStream2,
    prefixed_args: TokenStream2,
}

impl Codegen {
    fn new(args: &Args, level: &Level) -> Self {
        if args.prefixed_fields.is_empty() && args.formatting_args.is_empty() {
            let fmt_str = args
                .format_string
                .as_ref()
                .map(|s| s.value())
                .unwrap_or_else(String::new);
            let metadata = quote! {
                static META: quicklog::queue::Metadata = quicklog::queue::Metadata::new(
                    std::module_path!(),
                    std::file!(),
                    std::line!(),
                    #level,
                    #fmt_str,
                );
            };

            return Self {
                prologue: quote! {
                    let mut logger = quicklog::logger();
                    let now = quicklog::Quicklog::now();
                    let size = quicklog::queue::log_header_size();
                    let chunk = logger.prepare_write().start_write(size)?;
                    let mut cursor = quicklog::queue::Cursor::new(chunk);

                    let header = quicklog::queue::LogHeader::new(&META, now, quicklog::queue::ArgsKind::Normal(0));
                    cursor.write(&header)?;
                },
                metadata,
                fmt_args: quote! {},
                prefixed_args: quote! {},
            };
        }

        let mut ident_gen = IdentGen::new();
        // Check if we need to format the format string and format arguments
        //
        // If there are format arguments or the format string has format
        // specifiers (might be named captures), then we need to format it.
        // Otherwise, we can just pass the format string along with `Metadata`
        // and avoid one formatting operation.
        let original_fmt_str = args
            .format_string
            .as_ref()
            .map(|s| s.value())
            .unwrap_or_else(String::new);
        let has_fmt_args = !args.formatting_args.is_empty();
        let need_write_fmt_str = has_fmt_args || has_fmt_specifiers(original_fmt_str.as_str());
        let (fmt_args_alloc, fmt_args_write, mut fmt_str) = if need_write_fmt_str {
            let fmt_args = if has_fmt_args {
                let args = &args.formatting_args;
                quote! { , #args }
            } else {
                quote! {}
            };
            let ident = ident_gen.gen();

            // Entire format string + format args are wrapped into one argument
            (
                quote! {
                    let #ident = quicklog::queue::format_in(fmt_buffer, format_args!(#original_fmt_str #fmt_args));
                },
                quote! { cursor.write_str(#ident)?; },
                "{}".to_string(),
            )
        } else {
            (quote! {}, quote! {}, original_fmt_str)
        };

        // Format all prefixed args that needs to be eagerly formatted
        let prefixed_args_alloc: Vec<_> = args
            .prefixed_fields
            .iter()
            .filter_map(|f| {
                if !f.is_serialize() {
                    let arg = f.arg();
                    let formatter = f.formatter();
                    let ident = ident_gen.gen();
                    Some(quote! {
                        let #ident = quicklog::queue::format_in(fmt_buffer, format_args!(#formatter, #arg));
                    })
                } else {
                    None
                }
            })
            .collect();
        let eager_fmt = quote! {
            #fmt_args_alloc
            #(#prefixed_args_alloc)*
        };

        // After formatting, we just need to compute the required sizes for
        // Serialize args, and then we will know how much space we need from the
        // queue
        let fmt_idents = &ident_gen.0;
        let all_serialize = fmt_idents.is_empty();
        let get_total_sizes = (|| {
            let mut arg_sizes = Vec::new();

            if all_serialize {
                let serialize_args: Vec<_> = args.prefixed_fields.iter().map(|f| f.arg()).collect();
                return quote! {  quicklog::queue::log_header_size() + (#(&#serialize_args,)*).buffer_size_required() };
            }

            for ident in fmt_idents {
                arg_sizes.push(quote! { (quicklog::queue::LogArgType::Fmt, #ident.len()) });
            }

            for arg in args.prefixed_fields.iter().filter_map(|f| {
                if f.is_serialize() {
                    Some(f.arg())
                } else {
                    None
                }
            }) {
                arg_sizes.push(
                    quote! { (quicklog::queue::LogArgType::Serialize, #arg.buffer_size_required())},
                );
            }

            quote! { quicklog::queue::log_size_required(&[#(#arg_sizes),*]) }
        })();

        // Proceed with writing to the queue
        let (args_kind, prefixed_args_write): (TokenStream2, TokenStream2) = if all_serialize {
            // Optimized case: all arguments are `Serialize`. We skip writing
            // the argument header
            let args: Vec<_> = args
                .prefixed_fields
                .iter()
                .map(PrefixedField::arg)
                .collect();
            let args_kind =
                quote! { quicklog::queue::ArgsKind::AllSerialize(_decode_fn(&(#(&#args,)*))) };
            let write = quote! { cursor.write(&(#(&#args,)*))?; };

            (args_kind, write)
        } else {
            // Normal case: mix of `Serialize` and `Debug`/`Display`
            let num_args =
                args.prefixed_fields.len() + need_write_fmt_str.then_some(1).unwrap_or_default();
            let args_kind = quote! { quicklog::queue::ArgsKind::Normal(#num_args) };

            let mut ident_iter = fmt_idents.iter();
            if need_write_fmt_str {
                // First ident is fmt args, second onwards is prefixed args
                _ = ident_iter.next();
            }

            let write = args
                .prefixed_fields
                .iter()
                .filter_map(|field| match field {
                    PrefixedField::Unnamed(i) => match i {
                        PrefixedArg::Debug(_) | PrefixedArg::Display(_) => {
                            let ident = ident_iter.next()?;
                            Some(quote! { cursor.write_str(#ident)?; })
                        }
                        PrefixedArg::Serialize(a) => Some(quote! { cursor.write_serialize(&#a)?; }),
                    },
                    PrefixedField::Named(f) => match &f.arg {
                        PrefixedArg::Debug(_) | PrefixedArg::Display(_) => {
                            let ident = ident_iter.next()?;
                            Some(quote! { cursor.write_str(#ident)?; })
                        }
                        PrefixedArg::Serialize(a) => Some(quote! { cursor.write_serialize(&#a)?; }),
                    },
                })
                .collect();

            (args_kind, write)
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

        // Logging initializing steps: acquire logger and prepare all buffers
        // for writing to the queue
        let prologue = quote! {
            let mut logger = quicklog::logger();
            let now = quicklog::Quicklog::now();
            let mut state = logger.prepare_write();
            let fmt_buffer = state.fmt_buffer;
            #eager_fmt
            let size = #get_total_sizes;
            let chunk = state.start_write(size)?;
            let mut cursor = quicklog::queue::Cursor::new(chunk);

            let header = quicklog::queue::LogHeader::new(&META, now, #args_kind);
            cursor.write(&header)?;
        };

        // Metadata construction
        let metadata_write = quote! {
            static META: quicklog::queue::Metadata = quicklog::queue::Metadata::new(
                std::module_path!(),
                std::file!(),
                std::line!(),
                #level,
                #fmt_str,
            );
        };

        Self {
            prologue,
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

/// Parses token stream into the different components of `Args` and
/// generates required tokens from the inputs
pub(crate) fn expand(level: Level, input: TokenStream, defer_commit: bool) -> TokenStream {
    expand_parsed(level, parse_macro_input!(input as Args), defer_commit).into()
}

/// Main function for expanding the components parsed from the macro call
pub(crate) fn expand_parsed(level: Level, args: Args, defer_commit: bool) -> TokenStream2 {
    let Codegen {
        prologue,
        prefixed_args,
        fmt_args,
        metadata,
    } = Codegen::new(&args, &level);
    let finish = if defer_commit {
        quote! { logger.finish_write(commit_size); }
    } else {
        quote! { logger.finish_and_commit(commit_size); }
    };

    quote! {{
        if quicklog::is_level_enabled!(#level) {
            use quicklog::{serialize::Serialize};

            #metadata

            #[inline(always)]
            fn _decode_fn<T: quicklog::serialize::SerializeTpl>(_a: &T) -> quicklog::serialize::DecodeEachFn {
                T::decode_each
            }

            (|| {
                #prologue

                #fmt_args

                #prefixed_args

                let commit_size = cursor.finish();
                #finish

                Ok::<(), quicklog::queue::WriteError>(())
            })()
        } else {
            Ok(())
        }
        .unwrap_or(())
    }}
}
