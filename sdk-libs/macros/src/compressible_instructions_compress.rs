//! Helper functions for generating compress instruction code.

use quote::quote;
use syn::{Ident, ItemFn, ItemMod, Result};

use crate::compressible_instructions::InstructionVariant;

/// Generate CompressContext trait implementation.
pub fn generate_compress_context_impl(
    _variant: InstructionVariant,
    account_types: Vec<Ident>,
) -> Result<ItemMod> {
    let lifetime: syn::Lifetime = syn::parse_quote!('info);

    // Generate the compress_pda_account match arms for each account type
    let compress_arms: Vec<_> = account_types.iter().map(|name| {
        quote! {
            d if d == #name::LIGHT_DISCRIMINATOR => {
                drop(data);
                let data_borrow = account_info.try_borrow_data().map_err(|e| {
                    let err: anchor_lang::error::Error = e.into();
                    let program_error: anchor_lang::prelude::ProgramError = err.into();
                    let code = match program_error {
                        anchor_lang::prelude::ProgramError::Custom(code) => code,
                        _ => 0,
                    };
                    solana_program_error::ProgramError::Custom(code)
                })?;
                let mut account_data = #name::try_deserialize(&mut &data_borrow[..]).map_err(|e| {
                    let err: anchor_lang::error::Error = e.into();
                    let program_error: anchor_lang::prelude::ProgramError = err.into();
                    let code = match program_error {
                        anchor_lang::prelude::ProgramError::Custom(code) => code,
                        _ => 0,
                    };
                    solana_program_error::ProgramError::Custom(code)
                })?;
                drop(data_borrow);

                let compressed_info = light_sdk::compressible::compress_account::prepare_account_for_compression::<#name>(
                    program_id,
                    account_info,
                    &mut account_data,
                    meta,
                    cpi_accounts,
                    &compression_config.compression_delay,
                    &compression_config.address_space,
                )?;
                Ok(Some(compressed_info))
            }
        }
    }).collect();

    Ok(syn::parse_quote! {
        mod __compress_context_impl {
            use super::*;
            use light_sdk::LightDiscriminator;

            impl<#lifetime> light_sdk::compressible::CompressContext<#lifetime> for CompressAccountsIdempotent<#lifetime> {
                fn fee_payer(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                    &*self.fee_payer
                }

                fn config(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                    &self.config
                }

                fn rent_sponsor(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                    &self.rent_sponsor
                }

                fn ctoken_rent_sponsor(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                    &self.ctoken_rent_sponsor
                }

                fn compression_authority(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                    &self.compression_authority
                }

                fn ctoken_compression_authority(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                    &self.ctoken_compression_authority
                }

                fn ctoken_program(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                    &*self.ctoken_program
                }

                fn ctoken_cpi_authority(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                    &*self.ctoken_cpi_authority
                }

                fn compress_pda_account(
                    &self,
                    account_info: &solana_account_info::AccountInfo<#lifetime>,
                    meta: &light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                    cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'_, #lifetime>,
                    compression_config: &light_sdk::compressible::CompressibleConfig,
                    program_id: &solana_pubkey::Pubkey,
                ) -> std::result::Result<Option<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>, solana_program_error::ProgramError> {
                    let data = account_info.try_borrow_data().map_err(|e| {
                        let err: anchor_lang::error::Error = e.into();
                        let program_error: anchor_lang::prelude::ProgramError = err.into();
                        let code = match program_error {
                            anchor_lang::prelude::ProgramError::Custom(code) => code,
                            _ => 0,
                        };
                        solana_program_error::ProgramError::Custom(code)
                    })?;
                    let discriminator = &data[0..8];

                    match discriminator {
                        #(#compress_arms)*
                        _ => {
                            let err: anchor_lang::error::Error = anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch.into();
                            let program_error: anchor_lang::prelude::ProgramError = err.into();
                            let code = match program_error {
                                anchor_lang::prelude::ProgramError::Custom(code) => code,
                                _ => 0,
                            };
                            Err(solana_program_error::ProgramError::Custom(code))
                        }
                    }
                }
            }
        }
    })
}

/// Generate thin wrapper for compress processor.
///
/// Delegates to the runtime function in compressed-token-sdk.
pub fn generate_process_compress_accounts_idempotent(
    _variant: InstructionVariant,
) -> Result<ItemFn> {
    Ok(syn::parse_quote! {
        /// Core processor for compress_accounts_idempotent.
        ///
        /// Thin wrapper that delegates to compressed-token-sdk runtime.
        #[inline(never)]
        pub fn process_compress_accounts_idempotent<'info>(
            accounts: &CompressAccountsIdempotent<'info>,
            remaining_accounts: &[solana_account_info::AccountInfo<'info>],
            compressed_accounts: Vec<light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress>,
            signer_seeds: Vec<Vec<Vec<u8>>>,
            system_accounts_offset: u8,
        ) -> Result<()> {
            light_compressed_token_sdk::compress_runtime::process_compress_accounts_idempotent(
                accounts,
                remaining_accounts,
                compressed_accounts,
                signer_seeds,
                system_accounts_offset,
                LIGHT_CPI_SIGNER,
                &crate::ID,
            )
            .map_err(|e: solana_program_error::ProgramError| -> anchor_lang::error::Error { e.into() })
        }
    })
}

/// Generate Anchor entrypoint.
pub fn generate_compress_instruction_entrypoint(_variant: InstructionVariant) -> Result<ItemFn> {
    Ok(syn::parse_quote! {
        /// Auto-generated compress_accounts_idempotent instruction.
        #[inline(never)]
        pub fn compress_accounts_idempotent<'info>(
            ctx: Context<'_, '_, '_, 'info, CompressAccountsIdempotent<'info>>,
            proof: light_sdk::instruction::ValidityProof,
            compressed_accounts: Vec<light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress>,
            signer_seeds: Vec<Vec<Vec<u8>>>,
            system_accounts_offset: u8,
        ) -> Result<()> {
            __processor_functions::process_compress_accounts_idempotent(
                &ctx.accounts,
                &ctx.remaining_accounts,
                compressed_accounts,
                signer_seeds,
                system_accounts_offset,
            )
        }
    })
}
