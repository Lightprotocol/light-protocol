use anchor_lang::prelude::*;
use light_compressible::config::CompressibleConfig;

#[derive(Accounts)]
pub struct ClaimContext<'info> {
    /// Transaction authority (for wrapper access control)
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Forester PDA for tracking work
    #[account(mut)]
    pub registered_forester_pda: Account<'info, crate::epoch::register_epoch::ForesterEpochPda>,

    /// Pool PDA that receives the claimed rent (writable)
    /// CHECK: This account is validated in the compressed token program
    #[account(mut)]
    pub rent_recipient: AccountInfo<'info>,

    /// Rent authority PDA (derived from config)
    /// CHECK: PDA derivation is validated via has_one constraint
    pub rent_authority: AccountInfo<'info>,

    /// CompressibleConfig account
    /// CHECK: Validated in the compressed token program
    #[account(
        has_one = rent_authority,
        has_one = rent_recipient
    )]
    pub compressible_config: Account<'info, CompressibleConfig>,

    /// Compressed token program
    /// CHECK: Must be the compressed token program ID
    pub compressed_token_program: AccountInfo<'info>,
}

pub fn process_claim<'info>(ctx: &Context<'_, '_, '_, 'info, ClaimContext<'info>>) -> Result<()> {
    // Build instruction data: discriminator (107u8) + pool_pda_bump
    let mut instruction_data = vec![107u8]; // Claim instruction discriminator
    instruction_data.push(ctx.accounts.compressible_config.rent_recipient_bump);

    // Prepare CPI accounts in the exact order expected by claim processor
    let mut cpi_accounts = vec![
        ctx.accounts.rent_recipient.to_account_info(),
        ctx.accounts.rent_authority.to_account_info(),
        ctx.accounts.compressible_config.to_account_info(),
    ];
    let mut cpi_account_metas = vec![
        anchor_lang::solana_program::instruction::AccountMeta::new(
            ctx.accounts.compressible_config.rent_recipient,
            false,
        ),
        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
            ctx.accounts.compressible_config.rent_authority,
            true,
        ),
        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
            ctx.accounts.compressible_config.key(),
            false,
        ),
    ];

    // Add all remaining accounts (token accounts to claim from)
    for account in ctx.remaining_accounts.iter() {
        cpi_account_metas.push(AccountMeta::new(account.key(), false));
        cpi_accounts.push(account.to_account_info());
    }

    // Prepare signer seeds for rent_authority PDA
    // The rent_authority is derived as: [b"rent_authority", version, 0]
    let version_bytes = ctx.accounts.compressible_config.version.to_le_bytes();
    let rent_authority_bump = ctx.accounts.compressible_config.rent_authority_bump;
    let signer_seeds = &[
        b"rent_authority".as_slice(),
        version_bytes.as_slice(),
        &[rent_authority_bump],
    ];

    // Execute CPI with rent_authority PDA as signer
    anchor_lang::solana_program::program::invoke_signed(
        &anchor_lang::solana_program::instruction::Instruction {
            program_id: pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m"),
            accounts: cpi_account_metas,
            data: instruction_data,
        },
        &cpi_accounts,
        &[signer_seeds],
    )?;

    Ok(())
}
