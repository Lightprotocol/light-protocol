use anchor_lang::{
    prelude::*,
    solana_program::{program::invoke_signed, system_instruction},
};
use light_compressible::config::CompressibleConfig;

use crate::errors::RegistryError;

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
        has_one = rent_recipient
    )]
    pub compressible_config: Account<'info, CompressibleConfig>,

    /// The pool PDA (rent_recipient) that holds the funds
    /// CHECK: Validated via has_one
    #[account(mut)]
    pub rent_recipient: AccountInfo<'info>,

    /// The destination account to receive the withdrawn funds
    /// CHECK: Can be any account that can receive SOL
    #[account(mut)]
    pub destination: AccountInfo<'info>,

    /// System program for the transfer
    pub system_program: Program<'info, System>,
}

// TODO: need to revert the ctoken program removal and invoke it via cpi
// the rent recipient must sign the cpi to fund token account creation
pub fn process_withdraw_funding_pool(
    ctx: &Context<WithdrawFundingPool>,
    amount: u64,
) -> Result<()> {
    // Check that pool has sufficient funds
    let pool_lamports = ctx.accounts.rent_recipient.lamports();
    require!(pool_lamports > amount, RegistryError::InsufficientFunds);

    // Create system transfer instruction from rent_recipient PDA to destination
    let transfer_ix = system_instruction::transfer(
        &ctx.accounts.rent_recipient.key(),
        &ctx.accounts.destination.key(),
        amount,
    );
    // Get rent_recipient_bump from the config
    let pool_pda_bump = ctx.accounts.compressible_config.rent_recipient_bump;

    // The rent_recipient is a PDA derived as: [b"rent_recipient", version, 0]
    let version_bytes = ctx.accounts.compressible_config.version.to_le_bytes();
    let seeds = &[
        b"rent_recipient".as_slice(),
        version_bytes.as_slice(),
        &[0],
        &[pool_pda_bump],
    ];

    // Execute the transfer with the PDA as signer
    invoke_signed(
        &transfer_ix,
        &[
            ctx.accounts.rent_recipient.to_account_info(),
            ctx.accounts.destination.to_account_info(),
        ],
        &[seeds],
    )?;

    Ok(())
}
