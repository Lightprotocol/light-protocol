//! Compress code generation.
//!
//! This module provides the `CompressBuilder` for generating compress instruction
//! code including context implementation, processor, entrypoint, and accounts struct.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Result, Type};

use super::parsing::InstructionVariant;
use crate::light_pdas::shared_utils::qualify_type_with_crate;

// =============================================================================
// COMPRESS BUILDER
// =============================================================================

/// Information about a compressible account type.
#[derive(Clone)]
pub struct CompressibleAccountInfo {
    /// The account type.
    pub account_type: Type,
    /// True if the account uses zero-copy (Pod) serialization.
    pub is_zero_copy: bool,
}

/// Builder for generating compress instruction code.
///
/// Encapsulates the account types and variant configuration needed to generate
/// all compress-related code: context implementation, processor function,
/// instruction entrypoint, and accounts struct.
pub(super) struct CompressBuilder {
    /// Account types that can be compressed with their zero_copy flags.
    accounts: Vec<CompressibleAccountInfo>,
    /// The instruction variant (PdaOnly, TokenOnly, or Mixed).
    variant: InstructionVariant,
}

impl CompressBuilder {
    /// Create a new CompressBuilder with the given account infos and variant.
    ///
    /// # Arguments
    /// * `accounts` - The account types with their zero_copy flags
    /// * `variant` - The instruction variant determining what gets generated
    ///
    /// # Returns
    /// A new CompressBuilder instance
    pub fn new(accounts: Vec<CompressibleAccountInfo>, variant: InstructionVariant) -> Self {
        Self { accounts, variant }
    }

    // -------------------------------------------------------------------------
    // Query Methods
    // -------------------------------------------------------------------------

    /// Returns true if this builder generates PDA compression code.
    ///
    /// This is true for `PdaOnly` and `Mixed` variants.
    pub fn has_pdas(&self) -> bool {
        matches!(
            self.variant,
            InstructionVariant::PdaOnly | InstructionVariant::Mixed
        )
    }

    /// Validate the builder configuration.
    ///
    /// Checks that:
    /// - At least one account type is provided (for PDA variants)
    /// - All account sizes are within the 800-byte limit
    ///
    /// # Returns
    /// `Ok(())` if validation passes, or a `syn::Error` describing the issue.
    pub fn validate(&self) -> Result<()> {
        // For variants that include PDAs, require at least one account type
        if self.has_pdas() && self.accounts.is_empty() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "CompressBuilder requires at least one account type for PDA compression",
            ));
        }
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Code Generation Methods
    // -------------------------------------------------------------------------

    /// Generate the compress dispatch function.
    ///
    /// Creates a function matching `CompressDispatchFn` signature that handles
    /// discriminator-based deserialization and compression dispatch.
    /// This function is placed inside the processor module.
    pub fn generate_dispatch_fn(&self) -> Result<syn::ItemFn> {
        let compress_arms: Vec<_> = self.accounts.iter().map(|info| {
            let name = qualify_type_with_crate(&info.account_type);

            if info.is_zero_copy {
                // Pod (zero-copy) path: use bytemuck
                quote! {
                    d if d == #name::LIGHT_DISCRIMINATOR => {
                        let pod_bytes = &data[8..8 + core::mem::size_of::<#name>()];
                        let mut account_data: #name = *bytemuck::from_bytes(pod_bytes);
                        drop(data);
                        light_account::prepare_account_for_compression(
                            account_info, &mut account_data, meta, index, ctx,
                        )
                    }
                }
            } else {
                // Borsh path: use deserialize (not try_from_slice which requires all bytes consumed)
                // Anchor allocates INIT_SPACE (max size) but actual Borsh data may be shorter
                // due to variable-length fields (String, Vec), leaving trailing bytes.
                quote! {
                    d if d == #name::LIGHT_DISCRIMINATOR => {
                        let mut reader = &data[8..];
                        let mut account_data = #name::deserialize(&mut reader)
                            .map_err(|_| solana_program_error::ProgramError::InvalidAccountData)?;
                        drop(data);
                        light_account::prepare_account_for_compression(
                            account_info, &mut account_data, meta, index, ctx,
                        )
                    }
                }
            }
        }).collect();

        Ok(syn::parse_quote! {
            fn __compress_dispatch<'info>(
                account_info: &anchor_lang::prelude::AccountInfo<'info>,
                meta: &light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                index: usize,
                ctx: &mut light_account::CompressCtx<'_, 'info>,
            ) -> std::result::Result<(), solana_program_error::ProgramError> {
                use light_sdk::LightDiscriminator;
                use borsh::BorshDeserialize;
                let data = account_info.try_borrow_data()?;
                let discriminator: [u8; 8] = data[..8]
                    .try_into()
                    .map_err(|_| solana_program_error::ProgramError::InvalidAccountData)?;
                match discriminator {
                    #(#compress_arms)*
                    _ => Ok(()),
                }
            }
        })
    }

    /// Generate the processor function for compress accounts (v2 interface).
    pub fn generate_processor(&self) -> Result<syn::ItemFn> {
        Ok(syn::parse_quote! {
            #[inline(never)]
            pub fn process_compress_accounts_idempotent<'info>(
                remaining_accounts: &[solana_account_info::AccountInfo<'info>],
                instruction_data: &[u8],
            ) -> Result<()> {
                light_account::process_compress_pda_accounts_idempotent(
                    remaining_accounts,
                    instruction_data,
                    __compress_dispatch,
                    LIGHT_CPI_SIGNER,
                    &crate::ID,
                )
                .map_err(|e: solana_program_error::ProgramError| -> anchor_lang::error::Error { e.into() })
            }
        })
    }

    /// Generate the compress instruction entrypoint function (v2 interface).
    ///
    /// Accepts `instruction_data: Vec<u8>` as a single parameter.
    /// The SDK client wraps the serialized data in a Vec<u8> (4-byte length prefix),
    /// and Anchor deserializes Vec<u8> correctly with this format.
    pub fn generate_entrypoint(&self) -> Result<syn::ItemFn> {
        Ok(syn::parse_quote! {
            #[inline(never)]
            pub fn compress_accounts_idempotent<'info>(
                ctx: Context<'_, '_, '_, 'info, CompressAccountsIdempotent<'info>>,
                instruction_data: Vec<u8>,
            ) -> Result<()> {
                __processor_functions::process_compress_accounts_idempotent(
                    ctx.remaining_accounts,
                    &instruction_data,
                )
            }
        })
    }

    /// Generate the compress accounts struct and manual Anchor trait impls.
    ///
    /// Uses PhantomData for the `<'info>` lifetime so Anchor's CPI codegen
    /// can reference `CompressAccountsIdempotent<'info>`.
    /// All accounts are passed via remaining_accounts.
    pub fn generate_accounts_struct(&self) -> Result<syn::ItemStruct> {
        Ok(syn::parse_quote! {
            pub struct CompressAccountsIdempotent<'info>(
                std::marker::PhantomData<&'info ()>,
            );
        })
    }

    /// Generate manual Anchor trait implementations for the empty accounts struct.
    pub fn generate_accounts_trait_impls(&self) -> Result<TokenStream> {
        Ok(quote! {
            impl<'info> anchor_lang::Accounts<'info, CompressAccountsIdempotentBumps>
                for CompressAccountsIdempotent<'info>
            {
                fn try_accounts(
                    _program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                    _accounts: &mut &'info [anchor_lang::solana_program::account_info::AccountInfo<'info>],
                    _ix_data: &[u8],
                    _bumps: &mut CompressAccountsIdempotentBumps,
                    _reallocs: &mut std::collections::BTreeSet<anchor_lang::solana_program::pubkey::Pubkey>,
                ) -> anchor_lang::Result<Self> {
                    Ok(CompressAccountsIdempotent(std::marker::PhantomData))
                }
            }

            #[derive(Debug, Default)]
            pub struct CompressAccountsIdempotentBumps {}

            impl<'info> anchor_lang::Bumps for CompressAccountsIdempotent<'info> {
                type Bumps = CompressAccountsIdempotentBumps;
            }

            impl<'info> anchor_lang::ToAccountInfos<'info> for CompressAccountsIdempotent<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                    Vec::new()
                }
            }

            impl<'info> anchor_lang::ToAccountMetas for CompressAccountsIdempotent<'info> {
                fn to_account_metas(
                    &self,
                    _is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    Vec::new()
                }
            }

            impl<'info> anchor_lang::AccountsExit<'info> for CompressAccountsIdempotent<'info> {
                fn exit(
                    &self,
                    _program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                ) -> anchor_lang::Result<()> {
                    Ok(())
                }
            }

            #[cfg(feature = "idl-build")]
            impl<'info> CompressAccountsIdempotent<'info> {
                pub fn __anchor_private_gen_idl_accounts(
                    _accounts: &mut std::collections::BTreeMap<
                        String,
                        anchor_lang::idl::types::IdlAccount,
                    >,
                    _types: &mut std::collections::BTreeMap<
                        String,
                        anchor_lang::idl::types::IdlTypeDef,
                    >,
                ) -> Vec<anchor_lang::idl::types::IdlInstructionAccountItem> {
                    Vec::new()
                }
            }

            pub(crate) mod __client_accounts_compress_accounts_idempotent {
                use super::*;
                pub struct CompressAccountsIdempotent<'info>(
                    std::marker::PhantomData<&'info ()>,
                );
                impl<'info> borsh::ser::BorshSerialize for CompressAccountsIdempotent<'info> {
                    fn serialize<W: borsh::maybestd::io::Write>(
                        &self,
                        _writer: &mut W,
                    ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                        Ok(())
                    }
                }
                impl<'info> anchor_lang::ToAccountMetas for CompressAccountsIdempotent<'info> {
                    fn to_account_metas(
                        &self,
                        _is_signer: Option<bool>,
                    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                        Vec::new()
                    }
                }
            }

            pub(crate) mod __cpi_client_accounts_compress_accounts_idempotent {
                use super::*;
                pub struct CompressAccountsIdempotent<'info>(
                    std::marker::PhantomData<&'info ()>,
                );
                impl<'info> anchor_lang::ToAccountMetas for CompressAccountsIdempotent<'info> {
                    fn to_account_metas(
                        &self,
                        _is_signer: Option<bool>,
                    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                        Vec::new()
                    }
                }
                impl<'info> anchor_lang::ToAccountInfos<'info> for CompressAccountsIdempotent<'info> {
                    fn to_account_infos(
                        &self,
                    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                        Vec::new()
                    }
                }
            }
        })
    }

    /// Generate compress dispatch as an associated function on the enum.
    ///
    /// When `#[derive(LightProgram)]` is used, the dispatch function is generated
    /// as `impl EnumName { pub fn compress_dispatch(...) }` so it can be referenced
    /// as `EnumName::compress_dispatch` and passed to SDK functions.
    pub fn generate_enum_dispatch_method(&self, enum_name: &syn::Ident) -> Result<TokenStream> {
        let compress_arms: Vec<_> = self.accounts.iter().map(|info| {
            let name = qualify_type_with_crate(&info.account_type);

            if info.is_zero_copy {
                quote! {
                    d if d == #name::LIGHT_DISCRIMINATOR => {
                        let pod_bytes = &data[8..8 + core::mem::size_of::<#name>()];
                        let mut account_data: #name = *bytemuck::from_bytes(pod_bytes);
                        drop(data);
                        light_account::prepare_account_for_compression(
                            account_info, &mut account_data, meta, index, ctx,
                        )
                    }
                }
            } else {
                quote! {
                    d if d == #name::LIGHT_DISCRIMINATOR => {
                        let mut reader = &data[8..];
                        let mut account_data = #name::deserialize(&mut reader)
                            .map_err(|_| solana_program_error::ProgramError::InvalidAccountData)?;
                        drop(data);
                        light_account::prepare_account_for_compression(
                            account_info, &mut account_data, meta, index, ctx,
                        )
                    }
                }
            }
        }).collect();

        Ok(quote! {
            impl #enum_name {
                pub fn compress_dispatch<'info>(
                    account_info: &anchor_lang::prelude::AccountInfo<'info>,
                    meta: &light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                    index: usize,
                    ctx: &mut light_account::CompressCtx<'_, 'info>,
                ) -> std::result::Result<(), solana_program_error::ProgramError> {
                    use light_sdk::LightDiscriminator;
                    use borsh::BorshDeserialize;
                    let data = account_info.try_borrow_data()?;
                    let discriminator: [u8; 8] = data[..8]
                        .try_into()
                        .map_err(|_| solana_program_error::ProgramError::InvalidAccountData)?;
                    match discriminator {
                        #(#compress_arms)*
                        _ => Ok(()),
                    }
                }
            }
        })
    }

    /// Generate compile-time size validation for compressed accounts.
    pub fn generate_size_validation(&self) -> Result<TokenStream> {
        let size_checks: Vec<_> = self.accounts.iter().map(|info| {
            let qualified_type = qualify_type_with_crate(&info.account_type);

            if info.is_zero_copy {
                // For Pod types, use core::mem::size_of for size calculation
                quote! {
                    const _: () = {
                        const COMPRESSED_SIZE: usize = 8 + core::mem::size_of::<#qualified_type>();
                        if COMPRESSED_SIZE > 800 {
                            panic!(concat!(
                                "Compressed account '", stringify!(#qualified_type), "' exceeds 800-byte compressible account size limit. If you need support for larger accounts, send a message to team@lightprotocol.com"
                            ));
                        }
                    };
                }
            } else {
                // For Borsh types, use CompressedInitSpace trait
                quote! {
                    const _: () = {
                        const COMPRESSED_SIZE: usize = 8 + <#qualified_type as light_account::compression_info::CompressedInitSpace>::COMPRESSED_INIT_SPACE;
                        if COMPRESSED_SIZE > 800 {
                            panic!(concat!(
                                "Compressed account '", stringify!(#qualified_type), "' exceeds 800-byte compressible account size limit. If you need support for larger accounts, send a message to team@lightprotocol.com"
                            ));
                        }
                    };
                }
            }
        }).collect();

        Ok(quote! { #(#size_checks)* })
    }

    /// Generate the error codes enum.
    ///
    /// The error codes enum is the same for all variants. It includes all
    /// possible error conditions even if some don't apply to specific variants.
    /// This ensures consistent error handling across different instruction types.
    pub fn generate_error_codes(&self) -> Result<TokenStream> {
        // All variants use the same error codes - shared infrastructure
        // that covers all possible error conditions.
        Ok(quote! {
            #[error_code]
            pub enum LightInstructionError {
                #[msg("Rent sponsor mismatch")]
                InvalidRentSponsor,
                #[msg("Missing seed account")]
                MissingSeedAccount,
                #[msg("Seed value does not match account data")]
                SeedMismatch,
                #[msg("Not implemented")]
                CTokenDecompressionNotImplemented,
                #[msg("Not implemented")]
                PdaDecompressionNotImplemented,
                #[msg("Not implemented")]
                TokenCompressionNotImplemented,
                #[msg("Not implemented")]
                PdaCompressionNotImplemented,
            }
        })
    }
}
