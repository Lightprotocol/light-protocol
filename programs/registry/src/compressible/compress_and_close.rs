use anchor_lang::prelude::*;
use light_compressible::config::CompressibleConfig;

use crate::{
    compressible::compressed_token::{
        compress_and_close_ctoken_accounts_with_indices, CompressAndCloseIndices,
        Transfer2CpiAccounts,
    },
    errors::RegistryError,
};

#[derive(Accounts)]
pub struct CompressAndCloseContext<'info> {
    /// Transaction authority (for wrapper access control)
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Forester PDA for tracking work
    #[account(mut)]
    pub registered_forester_pda: Account<'info, crate::epoch::register_epoch::ForesterEpochPda>,

    /// Rent authority PDA (derived from config)
    /// CHECK: PDA derivation is validated via has_one constraint
    #[account(mut)]
    pub compression_authority: AccountInfo<'info>,

    /// CompressibleConfig account
    #[account(
        has_one = compression_authority
    )]
    pub compressible_config: Account<'info, CompressibleConfig>,
}

pub fn process_compress_and_close<'c: 'info, 'info>(
    ctx: &Context<'_, '_, 'c, 'info, CompressAndCloseContext<'info>>,
    authority_index: u8,
    destination_index: u8,
    indices: Vec<CompressAndCloseIndices>,
) -> Result<()> {
    // Validate config is not inactive (active or deprecated allowed for compress and close)
    ctx.accounts
        .compressible_config
        .validate_not_inactive()
        .map_err(ProgramError::from)?;

    // Validate indices
    require!(!indices.is_empty(), RegistryError::InvalidSigner);

    let fee_payer = ctx.accounts.authority.to_account_info();

    // Use Transfer2CpiAccounts to parse accounts
    let transfer2_accounts =
        Transfer2CpiAccounts::try_from_account_infos(fee_payer, ctx.remaining_accounts)
            .map_err(ProgramError::from)?;

    let instruction = compress_and_close_ctoken_accounts_with_indices(
        ctx.accounts.authority.key(),
        authority_index,
        destination_index,
        &indices,
        &transfer2_accounts.packed_accounts,
    )?;

    // Prepare signer seeds for compression_authority PDA
    let version_bytes = ctx.accounts.compressible_config.version.to_le_bytes();
    let compression_authority_bump = ctx.accounts.compressible_config.compression_authority_bump;
    let signer_seeds = &[
        b"compression_authority".as_slice(),
        version_bytes.as_slice(),
        &[compression_authority_bump],
    ];

    // Execute CPI with compression_authority PDA as signer
    anchor_lang::solana_program::program::invoke_signed(
        &instruction,
        transfer2_accounts.to_account_infos().as_slice(),
        &[signer_seeds],
    )?;

    Ok(())
}
