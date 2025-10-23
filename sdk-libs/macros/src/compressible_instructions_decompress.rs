//! Helper functions for generating decompress instruction code.
use syn::{Ident, ItemFn, ItemMod, Result};

use crate::compressible_instructions::InstructionVariant;

/// Generate the DecompressContext trait implementation for add_compressible_instructions.
///
/// This delegates to the shared implementation in derive_decompress_context module.
pub fn generate_decompress_context_impl(
    _variant: InstructionVariant,
    pda_type_idents: Vec<Ident>,
    token_variant_ident: Ident,
) -> Result<ItemMod> {
    // Use hardcoded 'info lifetime
    let lifetime: syn::Lifetime = syn::parse_quote!('info);

    let trait_impl = crate::derive_decompress_context::generate_decompress_context_trait_impl(
        pda_type_idents,
        token_variant_ident,
        lifetime,
    )?;

    // Wrap in a module
    Ok(syn::parse_quote! {
        mod __decompress_context_impl {
            use super::*;

            #trait_impl
        }
    })
}

/// Generate thin wrapper that calls the SDK's generic processor.
pub fn generate_process_decompress_accounts_idempotent(
    _variant: InstructionVariant,
) -> Result<ItemFn> {
    Ok(syn::parse_quote! {
        #[inline(never)]
        pub fn process_decompress_accounts_idempotent<'info>(
            accounts: &DecompressAccountsIdempotent<'info>,
            remaining_accounts: &[solana_account_info::AccountInfo<'info>],
            proof: light_sdk::instruction::ValidityProof,
            compressed_accounts: Vec<CompressedAccountData>,
            system_accounts_offset: u8,
        ) -> Result<()> {
            light_sdk::compressible::process_decompress_accounts_idempotent(
                accounts,
                remaining_accounts,
                compressed_accounts,
                proof,
                system_accounts_offset,
                LIGHT_CPI_SIGNER,
                &crate::ID,
            )
            .map_err(|e: solana_program_error::ProgramError| -> anchor_lang::error::Error { e.into() })
        }
    })
}

/// Generate the Anchor entrypoint as thin wrapper.
pub fn generate_decompress_instruction_entrypoint(_variant: InstructionVariant) -> Result<ItemFn> {
    Ok(syn::parse_quote! {
        /// Auto-generated decompress_accounts_idempotent instruction.
        #[inline(never)]
        pub fn decompress_accounts_idempotent<'info>(
            ctx: Context<'_, '_, '_, 'info, DecompressAccountsIdempotent<'info>>,
            proof: light_sdk::instruction::ValidityProof,
            compressed_accounts: Vec<CompressedAccountData>,
            system_accounts_offset: u8,
        ) -> Result<()> {
            __processor_functions::process_decompress_accounts_idempotent(
                &ctx.accounts,
                &ctx.remaining_accounts,
                proof,
                compressed_accounts,
                system_accounts_offset,
            )
        }
    })
}
