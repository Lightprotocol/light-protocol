//! Shared utility functions for proc macros.

use proc_macro::TokenStream;
use syn::Result;

/// Converts a `syn::Result<proc_macro2::TokenStream>` to `proc_macro::TokenStream`.
///
/// ## Usage
/// ```ignore
/// #[proc_macro_derive(MyMacro)]
/// pub fn my_macro(input: TokenStream) -> TokenStream {
///     let input = parse_macro_input!(input as DeriveInput);
///     into_token_stream(some_function(input))
/// }
/// ```
#[inline]
pub(crate) fn into_token_stream(result: Result<proc_macro2::TokenStream>) -> TokenStream {
    result.unwrap_or_else(|err| err.to_compile_error()).into()
}
