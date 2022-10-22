use crate::config;
use crate::errors::ErrorCode;
use crate::RegisteredVerifier;
use anchor_lang::prelude::*;
use std::ops::DerefMut;

#[derive(Accounts)]
pub struct WithdrawSol<'info> {
    /// CHECK:` Signer is registered verifier program.
    #[account(mut, seeds=[program_id.to_bytes().as_ref()],bump,seeds::program=registered_verifier_pda.pubkey)]
    pub authority: Signer<'info>,
    /// CHECK:` That the merkle tree token belongs to a registered Merkle tree.
    #[account(mut)]
    pub merkle_tree_token: AccountInfo<'info>,
    #[account(seeds=[&registered_verifier_pda.pubkey.to_bytes()],  bump)]
    pub registered_verifier_pda: Account<'info, RegisteredVerifier>,
    /// CHECK:`
    #[account(mut)]
    pub recipient: AccountInfo<'info>,
}

/// Transferring sol from the merkle_tree_token_pda to recipients which are passed-in
/// as remaining accounts.
pub fn process_sol_transfer<'info>(
    ctx: Context<'_, '_, '_, 'info, WithdrawSol<'info>>,
    amount: u64,
) -> Result<()> {
    msg!("Withdrawing sol {}", amount);
    sol_transfer(
        &ctx.accounts.merkle_tree_token.to_account_info(),
        &ctx.accounts.recipient.to_account_info(),
        amount,
    )
}

pub fn sol_transfer(
    from_account: &AccountInfo,
    dest_account: &AccountInfo,
    amount: u64,
) -> Result<()> {
    let from_starting_lamports = from_account.lamports();
    msg!("from_starting_lamports: {}", from_starting_lamports);
    let res = from_starting_lamports
        .checked_sub(amount)
        .ok_or(ProgramError::InvalidAccountData)?;
    **from_account.lamports.borrow_mut() = from_starting_lamports
        .checked_sub(amount)
        .ok_or(ProgramError::InvalidAccountData)?;
    msg!("from_ending_lamports: {}", res);

    let dest_starting_lamports = dest_account.lamports();
    **dest_account.lamports.borrow_mut() = dest_starting_lamports
        .checked_add(amount)
        .ok_or(ProgramError::InvalidAccountData)?;
    let res = dest_starting_lamports
        .checked_add(amount)
        .ok_or(ProgramError::InvalidAccountData)?;
    msg!("dest_starting_lamports: {}", dest_starting_lamports);
    msg!("dest_res_lamports: {}", res);

    Ok(())
}
