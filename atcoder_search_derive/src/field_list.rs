use crate::helper;
use proc_macro2::TokenStream;
use syn::DeriveInput;

pub fn impl_field_list(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input.into()).expect("failed to parse input token stream");

    let struct_name = &ast.ident;
    let fields = helper::extract_fields(&ast.data)
        .named
        .iter()
        .filter_map(|field| {
            field
                .ident
                .to_owned()
                .and_then(|ident| Some(ident.to_string()))
        })
        .collect::<Vec<String>>();
    let field_list = fields.join(",");

    quote::quote! {
        impl FieldList for #struct_name {
            fn field_list(&self) -> &'static str {
                #field_list
            }
        }
    }
}
