use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::parse_macro_input;

use crate::args::{Args, ExprFields};
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

enum CodegenError {
    Parsing(String),
}

impl ToTokens for CodegenError {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            CodegenError::Parsing(msg) => {
                let s = format!("parsing error: {}", msg);
                tokens.extend(quote! { #s });
            }
        }
    }
}

#[derive(Copy, Clone)]
enum ArgType {
    Fmt,
    Serialize,
}

struct LogArg {
    ty: ArgType,
    token: TokenStream2,
}

impl LogArg {
    fn new(ty: ArgType, token: TokenStream2) -> Self {
        Self { ty, token }
    }
}

struct Codegen {
    prologue: TokenStream2,
    write: TokenStream2,
    metadata: TokenStream2,
    fast_path: bool,
}

impl Codegen {
    fn new(args: &Args, level: &Level) -> Result<Self, CodegenError> {
        let mut ident_gen = IdentGen::new();
        // Check if we need to format the format string and format arguments
        //
        // If the the format string has non-Serialize format specifiers, then
        // we need to format it. Otherwise, we can just pass the format string
        // along with `Metadata` and avoid one formatting operation.
        let original_fmt_str = args
            .format_string
            .as_ref()
            .map(|s| s.value())
            .unwrap_or_else(String::new);

        let mut args_alloc = Vec::new();
        let mut args_in_order: Vec<LogArg> = Vec::new();

        let fragments = extract_fmt_args(original_fmt_str.as_str(), &args.formatting_args)?;
        let mut fmt_str = match fragments {
            FmtFragments::Serialize((new_fmt_str, serialize_fmt_args)) => {
                args_in_order.extend(
                    std::iter::repeat(ArgType::Serialize)
                        .zip(serialize_fmt_args)
                        .map(|(ty, arg)| LogArg::new(ty, arg)),
                );

                new_fmt_str
            }
            FmtFragments::NonSerialize => {
                let fmt_args = {
                    let args = &args.formatting_args;
                    quote! { , #args }
                };
                let ident = ident_gen.gen();

                // Entire format string + format args are wrapped into one argument
                args_alloc.push(quote! {
                    let #ident = state.format_in(format_args!(#original_fmt_str #fmt_args));
                });
                args_in_order.push(LogArg::new(ArgType::Fmt, ident.into_token_stream()));

                "{}".to_string()
            }
            FmtFragments::None => original_fmt_str,
        };
        // Format all prefixed args that needs to be eagerly formatted
        for field in &args.prefixed_fields {
            if field.is_serialize() {
                args_in_order.push(LogArg::new(ArgType::Serialize, field.arg()));
                continue;
            }

            let arg = field.arg();
            let formatter = field.formatter();
            let ident = ident_gen.gen();

            args_alloc.push(quote! {
                let #ident = state.format_in(format_args!(#formatter, #arg));
            });
            args_in_order.push(LogArg::new(ArgType::Fmt, ident.into_token_stream()));
        }

        // After formatting, we just need to compute the required sizes for all
        // args, and then we will know how much space we need from the queue
        let all_serialize = args_in_order
            .iter()
            .all(|LogArg { ty, .. }| matches!(ty, ArgType::Serialize));
        let (get_total_sizes, (args_kind, write)) =
            Self::gen_sizes_and_write(&args_in_order, all_serialize);

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
            #(#args_alloc)*
            let size = #get_total_sizes;
            let mut state = state.start_write(size)?;

            let header = quicklog::queue::LogHeader::new(&META, now, #args_kind);
            state.write(&header);
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

        Ok(Self {
            prologue,
            write,
            metadata: metadata_write,
            fast_path: all_serialize,
        })
    }

    fn gen_sizes_and_write(
        all_args: &[LogArg],
        all_serialize: bool,
    ) -> (TokenStream2, (TokenStream2, TokenStream2)) {
        (
            Self::gen_sizes(all_args, all_serialize),
            Self::gen_write(all_args, all_serialize),
        )
    }

    /// Computing sizes for requesting buffer slice from the queue.
    fn gen_sizes(all_args: &[LogArg], all_serialize: bool) -> TokenStream2 {
        if all_args.is_empty() {
            return quote! { quicklog::queue::log_header_size() };
        } else if all_serialize {
            let args = all_args.iter().map(|arg| &arg.token);
            return quote! {  quicklog::queue::log_header_size() + (#(&#args,)*).buffer_size_required() };
        }

        let arg_sizes = all_args
            .iter()
            .map(|arg| {
                let arg_tok = &arg.token;
                match arg.ty {
                    ArgType::Fmt => {
                        quote! { (quicklog::queue::LogArgType::Fmt, #arg_tok.len()) }
                    }
                    ArgType::Serialize => {
                        quote! { (quicklog::queue::LogArgType::Serialize, #arg_tok.buffer_size_required())}
                    }
                }
            });

        quote! { quicklog::queue::log_size_required(&[#(#arg_sizes),*]) }
    }

    /// Writing to the queue.
    fn gen_write(all_args: &[LogArg], all_serialize: bool) -> (TokenStream2, TokenStream2) {
        if all_args.is_empty() {
            return (quote! { quicklog::queue::ArgsKind::Normal(0) }, quote! {});
        } else if all_serialize {
            // Optimized case: all arguments are `Serialize`. We skip writing
            // the argument header
            let args: Vec<&TokenStream2> = all_args.iter().map(|arg| &arg.token).collect();
            let args_kind =
                quote! { quicklog::queue::ArgsKind::AllSerialize(_decode_fn(&(#(&#args,)*))) };
            let write = quote! { state.write(&(#(&#args,)*)); };

            return (args_kind, write);
        }

        let num_args = all_args.len();
        let args_kind = quote! { quicklog::queue::ArgsKind::Normal(#num_args) };

        let write = all_args
            .iter()
            .map(|arg| {
                let arg_tok = &arg.token;
                match arg.ty {
                    ArgType::Fmt => quote! { state.write_str(#arg_tok); },
                    ArgType::Serialize => quote! {  state.write_serialize(&#arg_tok); },
                }
            })
            .collect();

        (args_kind, write)
    }
}

#[derive(Debug)]
enum FmtError {
    /// Cannot find positional/named argument corresponding to a specifier.
    MissingArgument(usize),
    /// Mix of `Serialize` and `Debug`/`Display` format specifiers
    MixedArguments,
}

impl From<FmtError> for CodegenError {
    fn from(value: FmtError) -> Self {
        match value {
            FmtError::MissingArgument(n) => Self::Parsing(format!("format argument for position {} missing", n)),
            FmtError::MixedArguments => Self::Parsing("mixing serialize and non-serialize format specifiers in the format string is not allowed".to_string())
        }
    }
}

#[derive(Debug)]
enum FmtFragments {
    /// All Serialize arguments.
    Serialize((String, Vec<TokenStream2>)),
    /// All Debug/Display arguments.
    NonSerialize,
    /// No formatting arguments.
    None,
}

fn extract_fmt_args(fmt_str: &str, fmt_args: &ExprFields) -> Result<FmtFragments, FmtError> {
    if fmt_str.is_empty() {
        return Ok(FmtFragments::None);
    }

    struct FmtCount {
        positional: usize,
        named: usize,
    }

    let mut serialize_args = Vec::new();
    let mut specifier_ranges = Vec::new();
    let mut fmt_count = FmtCount {
        positional: 0,
        named: 0,
    };
    let mut chars = fmt_str.char_indices();

    while let Some((idx, c)) = chars.next() {
        if c != '{' {
            continue;
        }

        let s = chars.as_str();
        if s.starts_with('{') {
            // Escaped '{{'
            chars.next();
            continue;
        }

        // Might have unmatched open bracket, so explicitly check for presence
        // of close bracket
        let Some(end_idx) = s.find('}') else {
            continue;
        };

        // Found a valid format specifier, now just check if is serialize or not
        if let Some(colon_idx) = s[..end_idx].find(":^") {
            specifier_ranges.push((idx, idx + end_idx));
            // Check for named capture (note: parsing within '{}' here)
            if colon_idx == 0 {
                // {:^} -> unnamed capture
                let Some(field) = fmt_args.iter().nth(fmt_count.positional) else {
                    return Err(FmtError::MissingArgument(fmt_count.positional));
                };
                let expr = &field.arg;

                serialize_args.push(quote! { #expr });
                fmt_count.positional += 1;
            } else {
                // {ident:^} -> named capture
                let ident = &s[..colon_idx];
                if let Some(expr) = fmt_args.iter().find_map(|field| {
                    let field_name = field
                        .name
                        .as_ref()
                        .map(|ident| ident.to_string())
                        .unwrap_or_default();

                    (field_name.as_str() == ident).then_some(&field.arg)
                }) {
                    // explicit assignment
                    serialize_args.push(quote! { #expr });
                } else {
                    // implicit capture -- note that need to convert this from
                    // string to expr
                    let ident = Ident::new(ident, Span::call_site());
                    serialize_args.push(quote! { #ident });
                }

                fmt_count.named += 1;
            }
        } else if !serialize_args.is_empty() {
            // Cannot mix serialize and non-serialize arguments
            return Err(FmtError::MixedArguments);
        } else {
            // Non-serialize fmt specifier, e.g. {:?}, {}
            // Not necessarily positional, but doesn't matter since we will
            // just end up formatting the whole string
            fmt_count.positional += 1;
        }
    }

    if serialize_args.is_empty() && fmt_count.positional + fmt_count.named > 0 {
        // No serialize fmt specifiers found
        return Ok(FmtFragments::NonSerialize);
    } else if fmt_count.positional == 0 && fmt_count.named == 0 {
        // No fmt specifiers found
        return Ok(FmtFragments::None);
    }

    // Replace serialize fmt specifiers with empty fmt specifiers which will be
    // filled in again on the flushing end
    let mut new_fmt_str = String::with_capacity(fmt_str.len());
    let mut previous_end = 0;
    for (start, end) in specifier_ranges {
        new_fmt_str.push_str(&fmt_str[previous_end..start]);
        new_fmt_str.push_str("{}");
        previous_end = end + 1;
    }
    if previous_end + 1 < fmt_str.len() {
        new_fmt_str.push_str(&fmt_str[(previous_end + 1)..]);
    }

    Ok(FmtFragments::Serialize((new_fmt_str, serialize_args)))
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
        write,
        metadata,
        fast_path,
    } = match Codegen::new(&args, &level) {
        Ok(c) => c,
        Err(e) => {
            return quote! {
                compile_error!(#e)
            }
        }
    };

    let log_level_check = match level {
        Level::Info => quote! {
            quicklog::utils::likely(quicklog::is_level_enabled!(#level))
        },
        Level::Trace | Level::Debug | Level::Warn | Level::Error => quote! {
            quicklog::utils::unlikely(quicklog::is_level_enabled!(#level))
        },
    };
    let finish = if defer_commit {
        quote! {
            let finished = state.finish();
            logger.finish_write(finished);
        }
    } else {
        quote! {
            let finished = state.finish();
            logger.finish_and_commit(finished);
        }
    };

    let log_body = quote! {
        || {
            #prologue

            #write

            #finish

            Ok::<(), quicklog::queue::QueueError>(())
        }
    };
    let log_wrapper = match level {
        Level::Info if fast_path => {
            quote! {
                (#log_body)()
            }
        }
        Level::Info | Level::Trace | Level::Debug | Level::Warn | Level::Error => {
            quote! {
                quicklog::log_wrapper(#log_body)
            }
        }
    };

    quote! {{
        if #log_level_check {
            use quicklog::{serialize::Serialize};

            #metadata

            #[inline(always)]
            fn _decode_fn<T: quicklog::serialize::SerializeTpl>(_a: &T) -> quicklog::serialize::DecodeEachFn {
                T::decode_each
            }

            #log_wrapper
        } else {
            Ok(())
        }
        .unwrap_or(())
    }}
}
