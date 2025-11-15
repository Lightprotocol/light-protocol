use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, Result, Type};

/// Generates Pack and Unpack trait implementations for compressible account types
///
/// For types with Pubkey fields, this also generates a PackedXxx struct where Pubkeys become u8 indices.
/// For types without Pubkeys, generates identity Pack/Unpack implementations.
///
/// Usage: #[derive(CompressiblePack)]
#[inline(never)]
pub fn derive_compressible_pack(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;
    let packed_struct_name = format_ident!("Packed{}", struct_name);

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    &input,
                    "CompressiblePack only supports structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                &input,
                "CompressiblePack only supports structs",
            ));
        }
    };

    // Check if this struct has any Pubkey fields that need packing
    let has_pubkey_fields = fields.iter().any(|field| {
        if let Type::Path(type_path) = &field.ty {
            if let Some(segment) = type_path.path.segments.last() {
                segment.ident == "Pubkey"
            } else {
                false
            }
        } else {
            false
        }
    });

    if has_pubkey_fields {
        // Generate PackedXxx struct and Pack/Unpack implementations for types with Pubkeys
        generate_with_packed_struct(struct_name, &packed_struct_name, fields)
    } else {
        // Generate identity Pack/Unpack implementations for types without Pubkeys
        generate_identity_pack_unpack(struct_name)
    }
}

#[inline(never)]
fn generate_with_packed_struct(
    struct_name: &syn::Ident,
    packed_struct_name: &syn::Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> Result<TokenStream> {
    // Generate fields for the packed struct
    let packed_fields = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;

        // Convert Pubkey fields to u8, keep others as-is
        let packed_type = if is_pubkey_type(field_type) {
            quote! { u8 }
        } else {
            quote! { #field_type }
        };

        quote! { pub #field_name: #packed_type }
    });

    // Generate the packed struct
    let packed_struct = quote! {
        #[derive(Debug, Clone, anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize)]
        pub struct #packed_struct_name {
            #(#packed_fields,)*
        }
    };

    // Generate Pack implementation for original struct
    let pack_field_assignments = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;

        if *field_name == "compression_info" {
            quote! { #field_name: None }
        } else if is_pubkey_type(field_type) {
            quote! { #field_name: remaining_accounts.insert_or_get(self.#field_name) }
        } else if is_copy_type(field_type) {
            quote! { #field_name: self.#field_name }
        } else {
            quote! { #field_name: self.#field_name.clone() }
        }
    });

    let pack_impl = quote! {
        impl light_sdk::compressible::Pack for #struct_name {
            type Packed = #packed_struct_name;

            #[inline(never)]
            fn pack(&self, remaining_accounts: &mut light_sdk::instruction::PackedAccounts) -> Self::Packed {
                // Use stack-efficient struct construction to minimize frame size
                #packed_struct_name {
                    #(#pack_field_assignments,)*
                }
            }
        }
    };

    // Generate Unpack implementation for original struct (identity)
    let unpack_impl_original = quote! {
        impl light_sdk::compressible::Unpack for #struct_name {
            type Unpacked = Self;

            #[inline(never)]
            fn unpack(
                &self,
                _remaining_accounts: &[anchor_lang::prelude::AccountInfo],
            ) -> std::result::Result<Self::Unpacked, anchor_lang::prelude::ProgramError> {
                Ok(self.clone())
            }
        }
    };

    // Generate Pack implementation for packed struct (identity)
    let pack_impl_packed = quote! {
        impl light_sdk::compressible::Pack for #packed_struct_name {
            type Packed = Self;

            #[inline(never)]
            fn pack(&self, _remaining_accounts: &mut light_sdk::instruction::PackedAccounts) -> Self::Packed {
                self.clone()
            }
        }
    };

    // Generate Unpack implementation for packed struct
    let unpack_field_assignments = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;

        if *field_name == "compression_info" {
            quote! { #field_name: None }
        } else if is_pubkey_type(field_type) {
            quote! {
                #field_name: *remaining_accounts[self.#field_name as usize].key
            }
        } else if is_copy_type(field_type) {
            quote! { #field_name: self.#field_name }
        } else {
            quote! { #field_name: self.#field_name.clone() }
        }
    });

    let unpack_impl_packed = quote! {
        impl light_sdk::compressible::Unpack for #packed_struct_name {
            type Unpacked = #struct_name;

            #[inline(never)]
            fn unpack(
                &self,
                remaining_accounts: &[anchor_lang::prelude::AccountInfo],
            ) -> std::result::Result<Self::Unpacked, anchor_lang::prelude::ProgramError> {
                // Use stack-efficient struct construction to minimize frame size
                Ok(#struct_name {
                    #(#unpack_field_assignments,)*
                })
            }
        }
    };

    let expanded = quote! {
        #packed_struct
        #pack_impl
        #unpack_impl_original
        #pack_impl_packed
        #unpack_impl_packed
    };

    Ok(expanded)
}

#[inline(never)]
fn generate_identity_pack_unpack(struct_name: &syn::Ident) -> Result<TokenStream> {
    let pack_impl = quote! {
        impl light_sdk::compressible::Pack for #struct_name {
            type Packed = Self;

            #[inline(never)]
            fn pack(&self, _remaining_accounts: &mut light_sdk::instruction::PackedAccounts) -> Self::Packed {
                self.clone()
            }
        }
    };

    let unpack_impl = quote! {
        impl light_sdk::compressible::Unpack for #struct_name {
            type Unpacked = Self;

            #[inline(never)]
            fn unpack(
                &self,
                _remaining_accounts: &[anchor_lang::prelude::AccountInfo],
            ) -> std::result::Result<Self::Unpacked, anchor_lang::prelude::ProgramError> {
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

/// Check if a type is Pubkey
#[inline(never)]
fn is_pubkey_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            segment.ident == "Pubkey"
        } else {
            false
        }
    } else {
        false
    }
}

/// Determines if a type is likely to be Copy (simple heuristic)
#[inline(never)]
fn is_copy_type(ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) => {
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
#[inline(never)]
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
