use anchor_lang::prelude::*;
use light_compressible::config::CompressibleConfig;

/// Context for withdrawing funds from compressed token pool
#[derive(Accounts)]
pub struct WithdrawFundingPool<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// Authority that can withdraw - must match the config's withdrawal_authority
    pub withdrawal_authority: Signer<'info>,

    /// The compressible config that contains the withdrawal authority and rent_recipient
    #[account(
        has_one = withdrawal_authority,
        has_one = rent_recipient,
        has_one = rent_authority
    )]
    pub compressible_config: Account<'info, CompressibleConfig>,

    /// The pool PDA (rent_recipient) that holds the funds
    /// CHECK: Validated via has_one
    #[account(mut)]
    pub rent_recipient: AccountInfo<'info>,

    /// Rent authority PDA (derived from config) that will sign the CPI
    /// CHECK: PDA derivation is validated via has_one constraint
    pub rent_authority: AccountInfo<'info>,

    /// The destination account to receive the withdrawn funds
    /// CHECK: Can be any account that can receive SOL
    #[account(mut)]
    pub destination: AccountInfo<'info>,

    /// System program for the transfer
    pub system_program: Program<'info, System>,

    /// Compressed token program
    /// CHECK: Must be the compressed token program ID
    pub compressed_token_program: AccountInfo<'info>,
}

pub fn process_withdraw_funding_pool(
    ctx: &Context<WithdrawFundingPool>,
    amount: u64,
) -> Result<()> {
    // Build instruction data: [discriminator(108), pool_pda_bump, amount]
    let mut instruction_data = vec![108u8]; // WithdrawFundingPool instruction discriminator
                                            //instruction_data.push(ctx.accounts.compressible_config.rent_recipient_bump);
    instruction_data.extend_from_slice(&amount.to_le_bytes());

    // Prepare CPI accounts in the exact order expected by withdraw processor
    let cpi_accounts = vec![
        ctx.accounts.rent_recipient.to_account_info(), // pool_pda
        ctx.accounts.rent_authority.to_account_info(), // authority (will be signed by registry)
        ctx.accounts.destination.to_account_info(),    // destination
        ctx.accounts.system_program.to_account_info(), // system_program
        ctx.accounts.compressible_config.to_account_info(), // config
    ];

    let cpi_account_metas = vec![
        anchor_lang::solana_program::instruction::AccountMeta::new(
            ctx.accounts.rent_recipient.key(),
            false,
        ),
        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
            ctx.accounts.rent_authority.key(),
            true, // rent_authority needs to be marked as signer
        ),
        anchor_lang::solana_program::instruction::AccountMeta::new(
            ctx.accounts.destination.key(),
            false,
        ),
        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
            ctx.accounts.system_program.key(),
            false,
        ),
        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
            ctx.accounts.compressible_config.key(),
            false,
        ),
    ];

    // Prepare signer seeds for rent_authority PDA
    // The rent_authority is derived as: [b"rent_authority", version, 0]
    let version_bytes = ctx.accounts.compressible_config.version.to_le_bytes();
    let rent_authority_bump = ctx.accounts.compressible_config.rent_authority_bump;
    let signer_seeds = &[
        b"rent_authority".as_slice(),
        version_bytes.as_slice(),
        &[0],
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
