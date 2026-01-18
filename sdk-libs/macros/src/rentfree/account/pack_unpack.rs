use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, Result};

use super::utils::{extract_fields_from_derive_input, is_copy_type, is_pubkey_type};

#[inline(never)]
pub fn derive_compressible_pack(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;
    let packed_struct_name = format_ident!("Packed{}", struct_name);
    let fields = extract_fields_from_derive_input(&input)?;

    let has_pubkey_fields = fields.iter().any(|field| is_pubkey_type(&field.ty));

    if has_pubkey_fields {
        generate_with_packed_struct(struct_name, &packed_struct_name, fields)
    } else {
        generate_identity_pack_unpack(struct_name)
    }
}

#[inline(never)]
fn generate_with_packed_struct(
    struct_name: &syn::Ident,
    packed_struct_name: &syn::Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> Result<TokenStream> {
    let packed_fields = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;

        let packed_type = if is_pubkey_type(field_type) {
            quote! { u8 }
        } else {
            quote! { #field_type }
        };

        quote! { pub #field_name: #packed_type }
    });

    let packed_struct = quote! {
        #[derive(Debug, Clone, anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize)]
        pub struct #packed_struct_name {
            #(#packed_fields,)*
        }
    };

    let pack_field_assignments = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;

        if *field_name == "compression_info" {
            quote! { #field_name: None }
        } else if is_pubkey_type(field_type) {
            // Use read-only since pubkey fields are references (owner, authority, etc.)
            // not accounts that need to be modified
            quote! { #field_name: remaining_accounts.insert_or_get_read_only(self.#field_name) }
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
                #packed_struct_name {
                    #(#pack_field_assignments,)*
                }
            }
        }
    };

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

    let pack_impl_packed = quote! {
        impl light_sdk::compressible::Pack for #packed_struct_name {
            type Packed = Self;

            #[inline(never)]
            fn pack(&self, _remaining_accounts: &mut light_sdk::instruction::PackedAccounts) -> Self::Packed {
                self.clone()
            }
        }
    };

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
    let packed_struct_name = format_ident!("Packed{}", struct_name);

    // Generate type alias for consistency - Packed{Name} = {Name}
    let type_alias = quote! {
        pub type #packed_struct_name = #struct_name;
    };

    let pack_impl = quote! {
        impl light_sdk::compressible::Pack for #struct_name {
            type Packed = #struct_name;

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
        #type_alias
        #pack_impl
        #unpack_impl
    };

    Ok(expanded)
}
