use anchor_lang::prelude::*;
use light_compressed_token_sdk::instructions::compress_and_close::CompressAndCloseIndices;
use light_compressible::config::CompressibleConfig;

use crate::errors::RegistryError;

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
    pub rent_authority: AccountInfo<'info>,

    /// CompressibleConfig account
    #[account(
        has_one = rent_authority
    )]
    pub compressible_config: Account<'info, CompressibleConfig>,

    /// Compressed token program
    /// CHECK: Must be the compressed token program ID
    pub compressed_token_program: AccountInfo<'info>,
}

pub fn process_compress_and_close<'info>(
    ctx: &Context<'_, '_, '_, 'info, CompressAndCloseContext<'info>>,
    indices: Vec<CompressAndCloseIndices>,
) -> Result<()> {
    // Validate config is not inactive (active or deprecated allowed for compress and close)
    ctx.accounts.compressible_config
        .validate_not_inactive()
        .map_err(ProgramError::from)?;

    // Validate indices
    require!(!indices.is_empty(), RegistryError::InvalidSigner);

    let fee_payer = ctx.accounts.authority.to_account_info();

    // Use the new Transfer2CpiAccounts to parse accounts
    let transfer2_accounts =
        light_compressed_token_sdk::instructions::transfer2::Transfer2CpiAccounts::try_from_account_infos(
            &fee_payer,
            ctx.remaining_accounts
        ).map_err(|_| ProgramError::InvalidAccountData)?;

    // Get the packed accounts from the parsed structure
    let packed_accounts = transfer2_accounts.packed_accounts();

    // Use the SDK's compress_and_close function with the provided indices
    // Use the authority as fee_payer
    let  instruction = light_compressed_token_sdk::instructions::compress_and_close::compress_and_close_ctoken_accounts_with_indices(
        ctx.accounts.authority.key(),
        true,
        None, // cpi_context_pubkey
        &indices,
        packed_accounts,
    ).map_err(ProgramError::from)?;

    // Prepare signer seeds for rent_authority PDA
    let version_bytes = ctx.accounts.compressible_config.version.to_le_bytes();
    let rent_authority_bump = ctx.accounts.compressible_config.rent_authority_bump;
    let signer_seeds = &[
        b"rent_authority".as_slice(),
        version_bytes.as_slice(),
        &[rent_authority_bump],
    ];

    // Execute CPI with rent_authority PDA as signer
    anchor_lang::solana_program::program::invoke_signed(
        &instruction,
        transfer2_accounts.to_account_infos().as_slice(),
        &[signer_seeds],
    )?;

    Ok(())
}
