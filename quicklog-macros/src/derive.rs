use std::fmt::Write;

use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, spanned::Spanned, Data, DataEnum, DataStruct, DeriveInput, Fields, Index,
    Type,
};

/// Generates a `quicklog` `Serialize` implementation for a user-defined type.
///
/// There is no new real logic in the generated `encode` and `decode` functions.
/// All this macro does is walk the fields of the user-defined type and invoke
/// the `encode` or `decode` method corresponding to the `Serialize`
/// implementation for the type of the field.
///
/// Struct example:
/// ```ignore
/// use quicklog::Serialize;
///
/// #[derive(Serialize)]
/// struct TestStruct {
///     a: usize,
///     b: i32,
///     c: u32,
/// }
///
/// // Generated code (slightly simplified)
/// impl quicklog::serialize::Serialize for TestStruct {
///     fn encode<'buf>(
///         &self,
///         write_buf: &'buf mut [u8],
///     ) -> (quicklog::serialize::Store<'buf>, &'buf mut [u8]) {
///         let (chunk, rest) = write_buf.split_at_mut(self.buffer_size_required());
///         let (_, mut tail) = chunk.split_at_mut(0);
///         let TestStruct { a, b, c } = self;
///         let (_, tail) = a.encode(tail);
///         let (_, tail) = b.encode(tail);
///         let (_, tail) = c.encode(tail);
///         assert!(tail.is_empty());
///         (quicklog::serialize::Store::new(Self::decode, chunk), rest)
///     }
///     fn decode(read_buf: &[u8]) -> (String, &[u8]) {
///         let (a, read_buf) = <usize as quicklog::serialize::Serialize>::decode(read_buf);
///         let (b, read_buf) = <i32 as quicklog::serialize::Serialize>::decode(read_buf);
///         let (c, read_buf) = <u32 as quicklog::serialize::Serialize>::decode(read_buf);
///         (
///             format!("TestStruct {{ a: {0}, b: {1}, c: {2} }}", a, b, c),
///             read_buf,
///         )
///     }
///     fn buffer_size_required(&self) -> usize {
///         let TestStruct { a, b, c } = self;
///         a.buffer_size_required() + b.buffer_size_required()
///             + c.buffer_size_required()
///     }
/// }
/// ```
///
/// The codegen for enums is slightly more involved due to having to wrap the
/// core logic in match arms. Also, we need to additionally encode the enum
/// variant, which we naively do here by simply encoding the index of the
/// variant. Overall, the generated code should look similar to that for
/// structs:
///
/// ```ignore
/// use quicklog::Serialize;
/// enum TestEnum {
///     Foo(String),
///     Bar { a: String, b: usize },
///     Baz(TestStruct),
/// }
///
/// // Generated code (slightly simplified)
/// impl quicklog::serialize::Serialize for TestEnum {
///     fn encode<'buf>(
///         &self,
///         write_buf: &'buf mut [u8],
///     ) -> (quicklog::serialize::Store<'buf>, &'buf mut [u8]) {
///         let (chunk, rest) = write_buf.split_at_mut(self.buffer_size_required());
///         let (_, mut tail) = chunk.split_at_mut(0);
///         match self {
///             Self::Foo(x) => {
///                 (_, tail) = (0 as usize).encode(tail);
///                 (_, tail) = x.encode(tail);
///             }
///             Self::Bar { a, b } => {
///                 (_, tail) = (1 as usize).encode(tail);
///                 (_, tail) = a.encode(tail);
///                 (_, tail) = b.encode(tail);
///             }
///             Self::Baz(x) => {
///                 (_, tail) = (2 as usize).encode(tail);
///                 (_, tail) = x.encode(tail);
///             }
///         }
///         assert!(tail.is_empty());
///         (quicklog::serialize::Store::new(Self::decode, chunk), rest)
///     }
///     fn decode(read_buf: &[u8]) -> (String, &[u8]) {
///         let (variant_type, read_buf) = <usize as quicklog::serialize::Serialize>::decode(
///             read_buf,
///         );
///         let variant_type = variant_type
///             .parse::<usize>()
///             .expect(format!("unknown variant type decoded from buffer: {}", variant_type).as_str());
///         match variant_type {
///             0 => {
///                 let (x, read_buf) = <String as quicklog::serialize::Serialize>::decode(
///                     read_buf,
///                 );
///                 (
///                     format!("Foo({0})", x),
///                     read_buf,
///                 )
///             }
///             1 => {
///                 let (a, read_buf) = <String as quicklog::serialize::Serialize>::decode(
///                     read_buf,
///                 );
///                 let (b, read_buf) = <usize as quicklog::serialize::Serialize>::decode(
///                     read_buf,
///                 );
///                 (
///                     format!("Bar {{ a: {0}, b: {1} }}", a, b),
///                     read_buf,
///                 )
///             }
///             2 => {
///                 let (x, read_buf) = <TestStruct as quicklog::serialize::Serialize>::decode(
///                     read_buf,
///                 );
///                 (
///                     format!("Baz({0})", x),
///                     read_buf,
///                 )
///             }
///             i => unimplemented!("unknown variant type decoded from buffer: {}", i),
///         }
///     }
///     fn buffer_size_required(&self) -> usize {
///         match self {
///             Self::Foo(x) => std::mem::size_of::<usize>() + x.buffer_size_required(),
///             Self::Bar { a, b } => std::mem::size_of::<usize>() + a.buffer_size_required() + b.buffer_size_required(),
///             Self::Baz(x) => std::mem::size_of::<usize>() + x.buffer_size_required(),
///         }
///     }
/// }
/// ```
pub(crate) fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ty_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let initial_split = quote! {
        let (chunk, rest) = write_buf.split_at_mut(self.buffer_size_required());
        let (_, mut tail) = chunk.split_at_mut(0);
    };
    let finish_store = quote! {
        assert!(tail.is_empty());
        (quicklog::serialize::Store::new(Self::decode, chunk), rest)
    };

    let (encode, decode, buffer_size_required) = match &input.data {
        Data::Struct(DataStruct { fields, .. }) => {
            let assigned_names = destructure_names(fields);
            let variant_delimiter_match_all = match fields {
                Fields::Unit => quote! {},
                Fields::Unnamed(_) => {
                    quote! { (#(#assigned_names),*) }
                }
                Fields::Named(_) => {
                    quote! { { #(#assigned_names),* } }
                }
            };
            let (encode, decode, buf) = gen_serialize_methods(ty_name, fields, &assigned_names);

            // For structs, destructure once at top-level
            // Technically, this is not *strictly* necessary, since we can do
            // `self.name_1.encode(...)` and so on. This is just for a bit
            // more consistency with the codegen for enums, since using `self`
            // doesn't apply for matched enum variants, and we need to
            // destructure to extract the containing variant data
            let final_encode = quote! {
                let #ty_name #variant_delimiter_match_all = self;
                #encode
            };
            let final_buf = quote! {
                let #ty_name #variant_delimiter_match_all = self;
                #buf
            };

            (final_encode, decode, final_buf)
        }
        Data::Enum(DataEnum { variants, .. }) => {
            let num_variants = variants.len();
            let mut variant_encode = Vec::with_capacity(num_variants);
            let mut variant_decode = Vec::with_capacity(num_variants);
            let mut variant_buf = Vec::with_capacity(num_variants);

            for (idx, variant) in variants.iter().enumerate() {
                let variant_name = &variant.ident;
                let variant_fields = &variant.fields;
                let assigned_names = destructure_names(variant_fields);
                let variant_delimiter_match_all = match variant_fields {
                    Fields::Unit => quote! {},
                    Fields::Unnamed(_) => {
                        quote! { (#(#assigned_names),*) }
                    }
                    Fields::Named(_) => {
                        quote! { { #(#assigned_names),* } }
                    }
                };

                // Every enum variant can be seen as a struct, so just generate
                // the same way as with structs
                let (encode, decode, buffer_size_required) =
                    gen_serialize_methods(variant_name, variant_fields, &assigned_names);

                // Note that for enums, we need to encode the variant as well
                // We do that by assigning the variants to an index, starting
                // from 0
                let i = Index::from(idx);
                // Wrap encode logic within a match arm
                let encode_variant = quote! {
                    Self::#variant_name #variant_delimiter_match_all => {
                        (_, tail) = (#i as usize).encode(tail);
                        #encode
                    }
                };

                // Dispatch on correct decode method based on parsed idx
                let decode_variant = quote! {
                    #i => { #decode }
                };

                // Wrap buf size logic within a match arm
                let encode_buf = quote! {
                    Self::#variant_name #variant_delimiter_match_all => {
                        // Additional bytes for enum tag
                        std::mem::size_of::<usize>() + #buffer_size_required
                    }
                };
                variant_encode.push(encode_variant);
                variant_decode.push(decode_variant);
                variant_buf.push(encode_buf);
            }

            // Wrap all previously constructed match arms under match stmt
            let final_encode = quote! {
                match self {
                    #(#variant_encode)*
                }
            };
            let final_decode = quote! {
                // Decode variant type from first few bytes
                let (variant_type, read_buf) = <usize as quicklog::serialize::Serialize>::decode(read_buf);
                let variant_type = variant_type.parse::<usize>()
                    .expect(format!("unknown variant type decoded from buffer: {}", variant_type).as_str());
                match variant_type {
                    #(#variant_decode)*
                    i => unimplemented!("unknown variant type decoded from buffer: {}", i),
                }
            };
            let final_buf = quote! {
                match self {
                    #(#variant_buf)*
                }
            };

            (final_encode, final_decode, final_buf)
        }
        _ => unimplemented!("Deriving Serialize only supported for enums and structs"),
    };

    quote! {
        impl #impl_generics quicklog::serialize::Serialize for #ty_name #ty_generics #where_clause {
            fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> (quicklog::serialize::Store<'buf>, &'buf mut [u8]) {
                #initial_split

                #encode

                #finish_store
            }

            fn decode(read_buf: &[u8]) -> (String, &[u8]) {
                #decode
            }

            fn buffer_size_required(&self) -> usize {
                #buffer_size_required
            }
        }
    }
    .into()
}

/// Generates the main body of `encode`, `decode`, and `buffer_size_required`.
///
/// For `encode` and `buffer_size_required`: this calls the method on every
/// (destructured) field.
/// For `decode`: this calls the `decode` implementation based on the type of
/// each field.
fn gen_serialize_methods(
    ident: &Ident,
    fields: &Fields,
    field_names: &[TokenStream2],
) -> (TokenStream2, TokenStream2, TokenStream2) {
    if matches!(fields, Fields::Unit) {
        // Unit structs/enum variants are zero-sized with nothing to encode
        let name = ident.to_string();
        return (
            quote! {},
            quote! {
                (#name.to_string(), read_buf)
            },
            quote! { 0 },
        );
    }

    // Sequentially encode by calling `encode` of each field
    let encode: TokenStream2 = field_names
        .iter()
        .map(|name| {
            quote! { (_, tail) = #name.encode(tail); }
        })
        .collect();

    // Combine decode implementations from all field types
    let decode: Vec<_> = fields
        .iter()
        .map(|field| field.ty.clone())
        .zip(field_names.iter())
        .map(|(mut ty, name)| {
            if let Type::Reference(ty_ref) = &mut ty {
                _ = ty_ref.lifetime.take();
                _ = ty_ref.mutability.take();
            }

            quote! {
                let (#name, read_buf) = <#ty as quicklog::serialize::Serialize>::decode(read_buf);
            }
        })
        .collect();
    let decode_fmt_str = construct_fmt_str(ident, fields);
    let decode_fmt = quote! {
        #(#decode)*
        (format!(#decode_fmt_str, #(#field_names),*), read_buf)
    };

    let buffer_size_required = if field_names.is_empty() {
        quote! { 0 }
    } else {
        quote! {
            #(#field_names.buffer_size_required())+*
        }
    };

    (encode, decode_fmt, buffer_size_required)
}

/// Returns a format string based on the type deriving `Serialize`,
/// similar to the standard [`std::fmt::Debug`] output format.
fn construct_fmt_str(ident: &Ident, fields: &Fields) -> String {
    let mut fmt_str = String::new();
    fmt_str.push_str(ident.to_string().as_str());

    match fields {
        Fields::Named(f) => {
            let num_fields = f.named.len();

            fmt_str.push_str(" {{ ");
            for (idx, field) in fields.iter().enumerate() {
                let name = field.ident.as_ref().unwrap();
                if idx < num_fields - 1 {
                    // String automatically resizes if not enough capacity
                    write!(&mut fmt_str, "{}: {{}}, ", name).unwrap();
                } else {
                    write!(&mut fmt_str, "{}: {{}}", name).unwrap();
                }
            }
            fmt_str.push_str(" }}");
        }
        Fields::Unnamed(f) => {
            let num_fields = f.unnamed.len();

            fmt_str.push('(');
            for idx in 0..num_fields {
                if idx < num_fields - 1 {
                    // String automatically resizes if not enough capacity
                    write!(&mut fmt_str, "{{}}, ").unwrap();
                } else {
                    write!(&mut fmt_str, "{{}}").unwrap();
                }
            }
            fmt_str.push(')');
        }
        Fields::Unit => {}
    }

    fmt_str
}

/// Returns sequence of `Ident`s that are used to destructure named or unnamed
/// fields.
///
/// For named fields, this is simply the name of each field.
/// - e.g. TestStruct { a: usize, b: usize } -> [a, b]
/// For unnamed fields, we generate repeating `x`s for each field.
/// - e.g. TestStruct(String, usize) -> [x, xx]
///
/// This is mainly to assign variables to the results of decoded outputs,
/// and also to assign variables against matched data in enum variants.
fn destructure_names(fields: &Fields) -> Vec<TokenStream2> {
    fields
        .iter()
        .enumerate()
        .map(|(idx, field)| {
            if let Some(i) = field.ident.as_ref() {
                i.into_token_stream()
            } else {
                Ident::new(&"x".repeat(idx + 1), field.span()).into_token_stream()
            }
        })
        .collect()
}
