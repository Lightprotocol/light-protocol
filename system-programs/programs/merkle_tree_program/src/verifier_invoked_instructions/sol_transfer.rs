use crate::RegisteredAssetPool;
use crate::RegisteredVerifier;
use anchor_lang::prelude::*;
#[derive(Accounts)]
pub struct UnshieldSol<'info> {
    /// CHECK:` Signer is registered verifier program.
    #[account(mut , seeds=[__program_id.to_bytes().as_ref()],bump,seeds::program=registered_verifier_pda.pubkey)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub merkle_tree_token: Account<'info, RegisteredAssetPool>,
    #[account(mut, seeds=[&registered_verifier_pda.pubkey.to_bytes()],  bump)]
    pub registered_verifier_pda: Account<'info, RegisteredVerifier>,
    /// CHECK:`
    #[account(mut)]
    pub recipient: UncheckedAccount<'info>,
}

pub fn process_sol_transfer(
    from_account: &AccountInfo,
    dest_account: &AccountInfo,
    amount: u64,
) -> Result<()> {
    let from_starting_lamports = from_account.lamports();
    // msg!("from_starting_lamports: {}", from_starting_lamports);
    from_starting_lamports
        .checked_sub(amount)
        .ok_or(ProgramError::InvalidAccountData)?;
    **from_account.lamports.borrow_mut() = from_starting_lamports
        .checked_sub(amount)
        .ok_or(ProgramError::InvalidAccountData)?;
    // msg!("from_ending_lamports: {}", res);

    let dest_starting_lamports = dest_account.lamports();
    **dest_account.lamports.borrow_mut() = dest_starting_lamports
        .checked_add(amount)
        .ok_or(ProgramError::InvalidAccountData)?;
    dest_starting_lamports
        .checked_add(amount)
        .ok_or(ProgramError::InvalidAccountData)?;
    // msg!("dest_starting_lamports: {}", dest_starting_lamports);
    // msg!("dest_res_lamports: {}", res);

    Ok(())
}
