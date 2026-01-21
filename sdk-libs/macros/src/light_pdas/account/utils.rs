//! Shared utility functions for compressible macro generation.

use syn::{
    punctuated::Punctuated, Data, DeriveInput, Field, Fields, GenericArgument, ItemStruct,
    PathArguments, Result, Token, Type,
};

/// Extracts named fields from an ItemStruct with proper error handling.
///
/// Returns an error if the struct doesn't have named fields.
pub(crate) fn extract_fields_from_item_struct(
    input: &ItemStruct,
) -> Result<&Punctuated<Field, Token![,]>> {
    match &input.fields {
        Fields::Named(fields) => Ok(&fields.named),
        _ => Err(syn::Error::new_spanned(
            input,
            "Only structs with named fields are supported",
        )),
    }
}

/// Extracts named fields from a DeriveInput with proper error handling.
///
/// Returns an error if the input is not a struct with named fields.
pub(crate) fn extract_fields_from_derive_input(
    input: &DeriveInput,
) -> Result<&Punctuated<Field, Token![,]>> {
    match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => Ok(&fields.named),
            _ => Err(syn::Error::new_spanned(
                input,
                "Only structs with named fields are supported",
            )),
        },
        _ => Err(syn::Error::new_spanned(input, "Only structs are supported")),
    }
}

/// Determines if a type is a Copy type (primitives, Pubkey, and Options of Copy types).
///
/// This is used to decide whether to use `.clone()` or direct copy during field assignments.
#[inline(never)]
pub(crate) fn is_copy_type(ty: &Type) -> bool {
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
        Type::Array(_) => true,
        _ => false,
    }
}

/// Checks if a type argument contains a Copy type (for generic types like Option<T>).
#[inline(never)]
pub(crate) fn has_copy_inner_type(args: &PathArguments) -> bool {
    match args {
        PathArguments::AngleBracketed(args) => args.args.iter().any(|arg| {
            if let GenericArgument::Type(ty) = arg {
                is_copy_type(ty)
            } else {
                false
            }
        }),
        _ => false,
    }
}

/// Determines if a type is specifically a Pubkey type.
#[inline(never)]
pub(crate) fn is_pubkey_type(ty: &Type) -> bool {
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

/// Generates placeholder TokenAccountVariant and PackedTokenAccountVariant enums.
///
/// This is used when no token accounts are specified in compressible instructions.
/// We use a placeholder variant since Rust doesn't support empty enums with #[repr(u8)].
pub(crate) fn generate_empty_ctoken_enum() -> proc_macro2::TokenStream {
    quote::quote! {
        #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize, Debug, Clone, Copy)]
        #[repr(u8)]
        pub enum TokenAccountVariant {
            /// Placeholder variant for programs without token accounts
            Empty = 0,
        }

        #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize, Debug, Clone, Copy)]
        #[repr(u8)]
        pub enum PackedTokenAccountVariant {
            /// Placeholder variant for programs without token accounts
            Empty = 0,
        }

        impl light_token::pack::Pack for TokenAccountVariant {
            type Packed = PackedTokenAccountVariant;
            fn pack(&self, _remaining_accounts: &mut light_sdk::instruction::PackedAccounts) -> std::result::Result<Self::Packed, solana_program_error::ProgramError> {
                Ok(PackedTokenAccountVariant::Empty)
            }
        }

        impl light_token::pack::Unpack for PackedTokenAccountVariant {
            type Unpacked = TokenAccountVariant;
            fn unpack(&self, _remaining_accounts: &[solana_account_info::AccountInfo]) -> std::result::Result<Self::Unpacked, solana_program_error::ProgramError> {
                Ok(TokenAccountVariant::Empty)
            }
        }

        impl light_sdk::interface::TokenSeedProvider for TokenAccountVariant {
            fn get_seeds(&self, _program_id: &Pubkey) -> std::result::Result<(Vec<Vec<u8>>, Pubkey), solana_program_error::ProgramError> {
                Err(solana_program_error::ProgramError::InvalidAccountData)
            }

            fn get_authority_seeds(&self, _program_id: &Pubkey) -> std::result::Result<(Vec<Vec<u8>>, Pubkey), solana_program_error::ProgramError> {
                Err(solana_program_error::ProgramError::InvalidAccountData)
            }
        }

        impl light_sdk::interface::IntoCTokenVariant<LightAccountVariant, light_token::compat::TokenData> for TokenAccountVariant {
            fn into_ctoken_variant(self, _token_data: light_token::compat::TokenData) -> LightAccountVariant {
                // This function should never be called for programs without token accounts.
                // The Empty variant only exists in mint-only programs (no PDAs).
                // For programs with PDAs but no tokens, this impl exists only to satisfy trait bounds.
                unreachable!("into_ctoken_variant called on program without token accounts")
            }
        }
    }
}
