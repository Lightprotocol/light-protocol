use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Data, DeriveInput, Expr, Fields, Ident, Result, Token,
};

/// Parse the compress_as attribute content
struct CompressAsFields {
    fields: Punctuated<CompressAsField, Token![,]>,
}

struct CompressAsField {
    name: Ident,
    value: Expr,
}

impl Parse for CompressAsField {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let value: Expr = input.parse()?;
        Ok(CompressAsField { name, value })
    }
}

impl Parse for CompressAsFields {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(CompressAsFields {
            fields: Punctuated::parse_terminated(input)?,
        })
    }
}

/// Generates HasCompressionInfo, Size, and CompressAs trait implementations for compressible account types
///
/// Supports optional compress_as attribute for custom compression behavior:
/// #[derive(Compressible)]
/// #[compress_as(start_time = 0, end_time = None)]
/// pub struct GameSession { ... }
///
/// Usage: #[derive(Compressible)]
pub fn derive_compressible(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;

    // Validate struct has compression_info field
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    &input,
                    "Compressible only supports structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                &input,
                "Compressible only supports structs",
            ));
        }
    };

    // Find the compression_info field
    let compression_info_field = fields.iter().find(|field| {
        field
            .ident
            .as_ref()
            .map(|ident| ident == "compression_info")
            .unwrap_or(false)
    });

    if compression_info_field.is_none() {
        return Err(syn::Error::new_spanned(
            struct_name,
            "Compressible requires a field named 'compression_info' of type Option<CompressionInfo>"
        ));
    }

    // Parse the compress_as attribute (optional)
    let compress_as_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("compress_as"));

    let compress_as_fields = if let Some(attr) = compress_as_attr {
        Some(attr.parse_args::<CompressAsFields>()?)
    } else {
        None
    };

    // Generate HasCompressionInfo implementation
    let has_compression_info_impl = quote! {
        impl light_sdk::compressible::HasCompressionInfo for #struct_name {
            fn compression_info(&self) -> &light_sdk::compressible::CompressionInfo {
                self.compression_info
                    .as_ref()
                    .expect("CompressionInfo must be Some on-chain")
            }

            fn compression_info_mut(&mut self) -> &mut light_sdk::compressible::CompressionInfo {
                self.compression_info
                    .as_mut()
                    .expect("CompressionInfo must be Some on-chain")
            }

            fn compression_info_mut_opt(&mut self) -> &mut Option<light_sdk::compressible::CompressionInfo> {
                &mut self.compression_info
            }

            fn set_compression_info_none(&mut self) {
                self.compression_info = None;
            }
        }
    };

    // Generate Size implementation
    let size_impl = quote! {
        impl light_sdk::account::Size for #struct_name {
            fn size(&self) -> usize {
                Self::LIGHT_DISCRIMINATOR.len() + Self::INIT_SPACE
            }
        }
    };

    // Generate CompressAs implementation
    let field_assignments = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();

        // ALWAYS set compression_info to None - this is required for compressed storage
        if field_name == "compression_info" {
            return quote! { #field_name: None };
        }

        // Check if this field is overridden in the compress_as attribute
        let override_field = compress_as_fields
            .as_ref()
            .and_then(|fields| fields.fields.iter().find(|f| f.name == *field_name));

        if let Some(override_field) = override_field {
            let override_value = &override_field.value;
            quote! { #field_name: #override_value }
        } else {
            // Keep the original value - determine how to clone/copy based on field type
            let field_type = &field.ty;
            if is_copy_type(field_type) {
                quote! { #field_name: self.#field_name }
            } else {
                quote! { #field_name: self.#field_name.clone() }
            }
        }
    });

    let compress_as_impl = quote! {
        impl light_sdk::compressible::CompressAs for #struct_name {
            type Output = Self;

            fn compress_as(&self) -> std::borrow::Cow<'_, Self::Output> {
                std::borrow::Cow::Owned(Self {
                    #(#field_assignments,)*
                })
            }
        }
    };

    // Compute a conservative compile-time compressed INIT_SPACE that accounts for fields overridden to None
    // Specifically, for fields of type Option<T> that are set to None via #[compress_as(field = None)]
    // (and for compression_info which is always set to None), we subtract the inner T's INIT_SPACE.
    // For inner types, we try to use known primitive sizes, arrays, or <T>::INIT_SPACE when available.
    fn inner_type_size_tokens(ty: &syn::Type) -> proc_macro2::TokenStream {
        use quote::quote;
        match ty {
            syn::Type::Path(type_path) => {
                if let Some(seg) = type_path.path.segments.last() {
                    let ident_str = seg.ident.to_string();
                    // Known primitives and common types
                    let primitive = match ident_str.as_str() {
                        "u8" => Some(quote! { 1 }),
                        "i8" => Some(quote! { 1 }),
                        "bool" => Some(quote! { 1 }),
                        "u16" => Some(quote! { 2 }),
                        "i16" => Some(quote! { 2 }),
                        "u32" => Some(quote! { 4 }),
                        "i32" => Some(quote! { 4 }),
                        "u64" => Some(quote! { 8 }),
                        "i64" => Some(quote! { 8 }),
                        "u128" => Some(quote! { 16 }),
                        "i128" => Some(quote! { 16 }),
                        "Pubkey" => Some(quote! { 32 }),
                        _ => None,
                    };
                    if let Some(sz) = primitive {
                        return sz;
                    }
                    // Fall back to type-level INIT_SPACE if present
                    let ty_ts = quote! { #type_path };
                    return quote! { <#ty_ts>::INIT_SPACE };
                }
                quote! { 0 }
            }
            syn::Type::Array(arr) => {
                let elem = &arr.elem;
                let len = &arr.len;
                let elem_sz = inner_type_size_tokens(elem);
                quote! { (#len as usize) * (#elem_sz) }
            }
            _ => {
                // Unknown/unsupported types: assume 0 saving to avoid compile errors
                quote! { 0 }
            }
        }
    }

    // Build tokens for total savings from fields explicitly set to None
    let mut savings_tokens: Vec<proc_macro2::TokenStream> = Vec::new();
    for field in fields.iter() {
        let field_name = field.ident.as_ref().unwrap();

        // Determine whether this field is overridden to None via #[compress_as] or is compression_info
        let mut overridden_to_none = field_name == "compression_info";
        if !overridden_to_none {
            if let Some(attrs) = &compress_as_fields {
                if let Some(over_attr) = attrs.fields.iter().find(|f| f.name == *field_name) {
                    if let syn::Expr::Path(ref p) = over_attr.value {
                        if let Some(last) = p.path.segments.last() {
                            if last.ident == "None" {
                                overridden_to_none = true;
                            }
                        }
                    }
                }
            }
        }

        if overridden_to_none {
            // Check that the field type is Option<Inner> and subtract Inner's INIT_SPACE
            if let syn::Type::Path(type_path) = &field.ty {
                if let Some(seg) = type_path.path.segments.last() {
                    if seg.ident == "Option" {
                        if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                            if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                                let inner_sz = inner_type_size_tokens(inner_ty);
                                savings_tokens.push(quote! { #inner_sz });
                            }
                        }
                    }
                }
            }
        }
    }

    let compressed_init_space_impl = {
        if savings_tokens.is_empty() {
            quote! {
                impl light_sdk::compressible::compression_info::CompressedInitSpace for #struct_name { const COMPRESSED_INIT_SPACE: usize = Self::INIT_SPACE; }
            }
        } else {
            quote! {
                impl light_sdk::compressible::compression_info::CompressedInitSpace for #struct_name { const COMPRESSED_INIT_SPACE: usize = Self::INIT_SPACE - (0 #( + #savings_tokens )*); }
            }
        }
    };

    let expanded = quote! {
        #has_compression_info_impl
        #size_impl
        #compress_as_impl
        #compressed_init_space_impl
    };

    Ok(expanded)
}

/// Determines if a type is likely to be Copy (simple heuristic)
fn is_copy_type(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                let type_name = segment.ident.to_string();
                matches!(
                    type_name.as_str(),
                    "u8" | "u16"
                        | "u32"
                        | "u64"
                        | "u128"
                        | "usize"
                        | "i8"
                        | "i16"
                        | "i32"
                        | "i64"
                        | "i128"
                        | "isize"
                        | "f32"
                        | "f64"
                        | "bool"
                        | "char"
                        | "Pubkey"
                ) || (type_name == "Option" && has_copy_inner_type(&segment.arguments))
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Check if Option<T> where T is Copy
fn has_copy_inner_type(args: &syn::PathArguments) -> bool {
    match args {
        syn::PathArguments::AngleBracketed(args) => args.args.iter().any(|arg| {
            if let syn::GenericArgument::Type(ty) = arg {
                is_copy_type(ty)
            } else {
                false
            }
        }),
        _ => false,
    }
}

#[allow(dead_code)]
fn generate_identity_pack_unpack(struct_name: &syn::Ident) -> Result<TokenStream> {
    let pack_impl = quote! {
        impl light_sdk::compressible::Pack for #struct_name {
            type Packed = Self;

            fn pack(&self, _remaining_accounts: &mut light_sdk::instruction::PackedAccounts) -> Self::Packed {
                self.clone()
            }
        }
    };

    let unpack_impl = quote! {
        impl light_sdk::compressible::Unpack for #struct_name {
            type Unpacked = Self;

            fn unpack(
                &self,
                _remaining_accounts: &[solana_account_info::AccountInfo],
            ) -> std::result::Result<Self::Unpacked, solana_program_error::ProgramError> {
                Ok(self.clone())
            }
        }
    };

    let expanded = quote! {
        #pack_impl
        #unpack_impl
    };

    Ok(expanded)
}
