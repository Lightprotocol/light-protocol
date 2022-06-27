use anchor_lang::prelude::*;
use crate::utils::create_pda::create_and_check_pda;
use crate::config;
use anchor_lang::solana_program;
use crate::errors::ErrorCode;

#[derive(Accounts)]
pub struct WithdrawSOL<'info> {
    /// CHECK:` should be , address = Pubkey::new(&MERKLE_TREE_SIGNER_AUTHORITY)
    #[account(mut, address=solana_program::pubkey::Pubkey::new(&config::REGISTERED_VERIFIER_KEY_ARRAY[0]))]
    pub authority: Signer<'info>,
    /// CHECK:` doc comment explaining why no checks through types are necessary., owner= Pubkey::new(b"2c54pLrGpQdGxJWUAoME6CReBrtDbsx5Tqx4nLZZo6av")
    #[account(mut)]
    pub merkle_tree_token: AccountInfo<'info>,
    // Recipients are specified in remaining accounts and checked in the verifier
}

pub fn process_sol_transfer(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<()>{
    const DEPOSIT: u8 = 1;
    const WITHDRAWAL: u8 = 2;
    let account = &mut accounts.iter();
    let signer_account = next_account_info(account)?;

    msg!("instruction_data[0] {}", instruction_data[0]);

    match instruction_data[0] {
        DEPOSIT => {
            let tmp_storage_pda = next_account_info(account)?;
            let system_program_account = next_account_info(account)?;
            let rent_sysvar_info = next_account_info(account)?;
            let rent = &Rent::from_account_info(rent_sysvar_info)?;
            let merkle_tree_pda_token = next_account_info(account)?;
            let user_ecrow_acc = next_account_info(account)?;

            let amount = u64::from_le_bytes(instruction_data[1..9].try_into().unwrap());
            msg!("Depositing {}", amount);
            create_and_check_pda(
                program_id,
                signer_account,
                user_ecrow_acc,
                system_program_account,
                rent,
                &tmp_storage_pda.key.to_bytes(),
                &b"escrow"[..],
                0,      //bytes
                amount, // amount
                true,   //rent_exempt
            )?;
            // Close escrow account to make deposit to shielded pool.
            close_account(user_ecrow_acc, merkle_tree_pda_token)
        }
        WITHDRAWAL => {
            let merkle_tree_pda_token = next_account_info(account)?;
            // withdraws amounts to accounts
            msg!("Entered withdrawal. {:?}", instruction_data[1..].chunks(8));
            for amount_u8 in instruction_data[1..].chunks(8) {
                let amount = u64::from_le_bytes(amount_u8.try_into().unwrap());
                let to = next_account_info(account)?;
                msg!("Withdrawing {}", amount);
                sol_transfer(merkle_tree_pda_token, to, amount).unwrap();
            }
            Ok(())
        }
        _ => err!(ErrorCode::WithdrawalFailed),
    }
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
    msg!("from_starting_lamports: {}", res);

    let dest_starting_lamports = dest_account.lamports();
    **dest_account.lamports.borrow_mut() = dest_starting_lamports
        .checked_add(amount)
        .ok_or(ProgramError::InvalidAccountData)?;
    let res = dest_starting_lamports
        .checked_add(amount)
        .ok_or(ProgramError::InvalidAccountData)?;
    msg!("from_starting_lamports: {}", res);

    Ok(())
}
use anchor_lang::error::Error;

pub fn close_account(
    account: &AccountInfo,
    dest_account: &AccountInfo,
) -> Result<()> {
    //close account by draining lamports
    let dest_starting_lamports = dest_account.lamports();
    **dest_account.lamports.borrow_mut() = dest_starting_lamports
        .checked_add(account.lamports())
        .ok_or(ErrorCode::CloseAccountFailed)?;
    **account.lamports.borrow_mut() = 0;
    Ok(())
}
