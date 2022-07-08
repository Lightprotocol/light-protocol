use crate::config;
use crate::errors::ErrorCode;
use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use std::ops::DerefMut;

#[derive(Accounts)]
#[instruction(data: Vec<u8>,_verifier_index: u64, _merkle_tree_index: u64)]
pub struct WithdrawSol<'info> {
    /// CHECK:` Signer is registered verifier program.
    #[account(mut, address=solana_program::pubkey::Pubkey::new(&config::REGISTERED_VERIFIER_KEY_ARRAY[usize::try_from(_verifier_index).unwrap()]))]
    pub authority: Signer<'info>,
    /// CHECK:` That the merkle tree token belongs to a registered Merkle tree.
    #[account(mut, constraint = merkle_tree_token.key() == Pubkey::new(&config::MERKLE_TREE_ACC_BYTES_ARRAY[usize::try_from(_merkle_tree_index).unwrap()].1))]
    pub merkle_tree_token: AccountInfo<'info>,
    // Recipients are specified in remaining accounts and checked in the verifier program.
}

/// Transferring sol from the merkle_tree_token_pda to recipients which are passed-in
/// as remaining accounts.
pub fn process_sol_transfer(accounts: &[AccountInfo], instruction_data: &[u8]) -> Result<()> {
    let account = &mut accounts.iter();
    let merkle_tree_pda_token = next_account_info(account)?;
    // withdraws amounts to accounts
    msg!("Entered withdrawal. {:?}", instruction_data.chunks(8));
    for amount_u8 in instruction_data.chunks(8) {
        let amount = u64::from_le_bytes(amount_u8.try_into().unwrap());
        let to = next_account_info(account)?;
        msg!("Withdrawing {}", amount);
        sol_transfer(merkle_tree_pda_token, to, amount).unwrap();
    }
    Ok(())
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

pub fn close_account(account: &AccountInfo, dest_account: &AccountInfo) -> Result<()> {
    //close account by draining lamports
    let dest_starting_lamports = dest_account.lamports();
    **dest_account.lamports.borrow_mut() = dest_starting_lamports
        .checked_add(account.lamports())
        .ok_or(ErrorCode::CloseAccountFailed)?;
    **account.lamports.borrow_mut() = 0;
    let mut data = account.try_borrow_mut_data()?;
    for byte in data.deref_mut().iter_mut() {
        *byte = 0;
    }
    Ok(())
}
