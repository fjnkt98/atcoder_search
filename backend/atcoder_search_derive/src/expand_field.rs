use crate::helper;
use proc_macro2::TokenStream;
use quote::format_ident;
use syn::{punctuated::Punctuated, AttrStyle, DeriveInput, Ident, Meta, Token};

pub fn impl_expand_field(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input.into()).expect("failed to parse input token stream");

    let struct_name = &ast.ident;
    let fields = helper::extract_fields(&ast.data);

    let setters = fields
        .named
        .iter()
        .map(|field| {
            let ident = &field.ident.to_owned().unwrap();
            let ty = &field.ty;
            let ident_str = ident.to_string();
            let attrs = &field.attrs;

            let suffixes = attrs
                .iter()
                .filter_map(|attr| {
                    if attr.path().is_ident("suffix") {
                        match attr.style {
                            AttrStyle::Outer => match &attr.meta {
                                Meta::List(metalist) => {
                                    let parser =
                                        Punctuated::<Ident, Token![,]>::parse_separated_nonempty;
                                    Some(
                                        metalist
                                            .parse_args_with(parser)
                                            .expect("couldn't parse field attribute")
                                            .iter()
                                            .cloned()
                                            .map(|suffix| suffix)
                                            .collect::<Vec<_>>(),
                                    )
                                }
                                _ => None,
                            },
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
                .flat_map(|s| s)
                .collect::<Vec<_>>();

            if suffixes.is_empty() {
                if helper::is_contained_by(ty, "DateTime") {
                    vec![quote::quote! {
                        #ident_str: self.#ident.with_timezone(&chrono::Utc).to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                    }]
                } else {
                    vec![quote::quote! {
                        #ident_str: self.#ident,
                    }]
                }
            } else {
                let mut expanded_field_assignations: Vec<TokenStream> = suffixes
                    .iter()
                    .map(|suffix| {
                        let suffixed_ident_str = format_ident!("{}__{}", ident, suffix).to_string();

                        quote::quote! {
                            #suffixed_ident_str: self.#ident,
                        }
                    })
                    .collect::<Vec<_>>();
                expanded_field_assignations.push(quote::quote! {
                    #ident_str: self.#ident,
                });
                expanded_field_assignations
            }
        })
        .flat_map(|s| s)
        .collect::<Vec<_>>();

    quote::quote! {
        impl ExpandField for #struct_name {
            fn expand(&self) -> serde_json::Value {
                serde_json::json!({
                    #(#setters)*
                })
            }
        }
    }
}
