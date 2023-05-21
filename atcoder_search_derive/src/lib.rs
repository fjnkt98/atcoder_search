mod expand_field;
mod field_list;
mod helper;

use expand_field::impl_expand_field;
use field_list::impl_field_list;
use proc_macro::TokenStream;

#[proc_macro_derive(FieldList)]
pub fn derive_field_list(input: TokenStream) -> TokenStream {
    impl_field_list(input.into()).into()
}

#[proc_macro_derive(ExpandField, attributes(suffix))]
pub fn derive_expand_field(input: TokenStream) -> TokenStream {
    impl_expand_field(input.into()).into()
}
