use std::fmt::Write;

use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Type};

/// Generates a `quicklog` `Serialize` implementation for a user-defined struct.
///
/// There is no new real logic in the generated `encode` and `decode` functions
/// for the struct. The macro simply walks every field of the struct and
/// sequentially calls `encode` or `decode` corresponding to the `Serialize`
/// implementation for the type of the field.
///
/// For instance:
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
///         let (_, chunk_rest) = self.a.encode(chunk);
///         let (_, chunk_rest) = self.b.encode(chunk_rest);
///         let (_, chunk_rest) = self.c.encode(chunk_rest);
///         assert!(chunk_rest.is_empty());
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
///         self.a.buffer_size_required() + self.b.buffer_size_required()
///             + self.c.buffer_size_required()
///     }
/// }
/// ```
pub(crate) fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let Data::Struct(DataStruct { fields, .. }) = input.data else {
        todo!("Deriving Serialize only supported for structs currently")
    };

    if fields.is_empty() {
        return quote! {}.into();
    }

    let field_names: Vec<_> = fields
        .iter()
        .filter_map(|field| field.ident.as_ref())
        .collect();

    // Sequentially encode
    let initial_split = quote! {
        let (chunk, rest) = write_buf.split_at_mut(self.buffer_size_required());
    };
    let encode: Vec<_> = field_names
        .iter()
        .enumerate()
        .map(|(idx, name)| {
            if idx == 0 {
                quote! {
                    let (_, chunk_rest) = self.#name.encode(chunk);
                }
            } else {
                quote! {
                    let (_, chunk_rest) = self.#name.encode(chunk_rest);
                }
            }
        })
        .collect();
    let finish_store = quote! {
        assert!(chunk_rest.is_empty());
        (quicklog::serialize::Store::new(Self::decode, chunk), rest)
    };

    // Combine decode implementations from all field types
    let field_tys: Vec<_> = fields
        .iter()
        .map(|field| {
            // Unwrap: safe since we checked that this macro is only for structs
            // which always have named fields
            let field_name = field.ident.as_ref().unwrap();
            let mut field_ty = field.ty.clone();
            if let Type::Reference(ty_ref) = &mut field_ty {
                _ = ty_ref.lifetime.take();
                _ = ty_ref.mutability.take();
            }
            let decoded_ident = Ident::new(format!("{}", field_name).as_str(), field_name.span());

            quote! {
                let (#decoded_ident, read_buf) = <#field_ty as quicklog::serialize::Serialize>::decode(read_buf);
            }
        })
        .collect();

    // Assuming that each field in the output should just be separated by a space
    let num_fields = field_names.len();
    let mut decode_fmt_str = String::new();
    decode_fmt_str.push_str(&struct_name.to_string());
    decode_fmt_str.push_str(" {{ ");
    for (idx, field_name) in field_names.iter().enumerate() {
        let name = field_name.to_string();
        if idx < num_fields - 1 {
            // String automatically resizes if not enough capacity
            write!(&mut decode_fmt_str, "{}: {{}}, ", name).unwrap();
        } else {
            write!(&mut decode_fmt_str, "{}: {{}} }}}}", name).unwrap();
        }
    }

    quote! {
        impl #impl_generics quicklog::serialize::Serialize for #struct_name #ty_generics #where_clause {
            fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> (quicklog::serialize::Store<'buf>, &'buf mut [u8]) {
                #initial_split

                #(#encode)*

                #finish_store
            }

            fn decode(read_buf: &[u8]) -> (String, &[u8]) {
                #(#field_tys)*

                (format!(#decode_fmt_str, #(#field_names),*), read_buf)
            }

            fn buffer_size_required(&self) -> usize {
                #(self.#field_names.buffer_size_required())+*
            }
        }
    }
    .into()
}
