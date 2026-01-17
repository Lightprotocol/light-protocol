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

/// Convert snake_case to CamelCase (e.g., user_record -> UserRecord)
pub(crate) fn snake_to_camel_case(s: &str) -> String {
    s.split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

/// Convert PascalCase/CamelCase to snake_case (e.g., UserRecord -> user_record)
pub(crate) fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}
