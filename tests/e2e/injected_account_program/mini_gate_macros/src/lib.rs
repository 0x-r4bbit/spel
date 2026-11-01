//! Pass-through, so rustc accepts the attributes the framework matches by name.

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn mini_gate(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn require_gate(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
