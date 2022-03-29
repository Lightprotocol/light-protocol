use crate::user_account::state::UserAccount;
use solana_program::{
    account_info::AccountInfo, msg, program_error::ProgramError, program_pack::Pack,
    pubkey::Pubkey, sysvar::rent::Rent,
};

pub fn initialize_user_account(
    account: &AccountInfo,
    pubkey_signer: Pubkey,
    rent: Rent,
) -> Result<(), ProgramError> {
    //check for rent exemption
    if !rent.is_exempt(**account.lamports.borrow(), account.data.borrow().len()) {
        msg!("Insufficient balance to initialize rent exempt user account.");
        return Err(ProgramError::AccountNotRentExempt);
    }

    //initialize
    let mut user_account_data = UserAccount::unpack(&account.data.borrow())?;
    user_account_data.owner_pubkey = pubkey_signer;
    UserAccount::pack_into_slice(&user_account_data, &mut account.data.borrow_mut());
    Ok(())
}

// pub fn modify_user_account(
//     account: &AccountInfo,
//     signer: Pubkey,
//     rent: Rent,
//     data: &[u8],
// ) -> Result<(), ProgramError> {
//     let mut user_account_data = UserAccount::unpack(&account.data.borrow())?;

//     if user_account_data.owner_pubkey != signer {
//         msg!("wrong signer");
//         return Err(ProgramError::InvalidArgument);
//     }
//     //check for rent exemption
//     if !rent.is_exempt(**account.lamports.borrow(), account.data.borrow().len()) {
//         msg!("User account is not active.");
//         return Err(ProgramError::AccountNotRentExempt);
//     }

//     for y in data.chunks(8 + SIZE_UTXO as usize) {
//         //first 8 bytes are index
//         let modifying_index = usize::from_be_bytes(y[0..8].try_into().unwrap());
//         //last 64 bytes are the utxo
//         let enc_utxo = &y[8..SIZE_UTXO as usize + 8];
//         for (i, x) in user_account_data.enc_utxos[modifying_index * SIZE_UTXO as usize
//             ..modifying_index * SIZE_UTXO as usize + SIZE_UTXO as usize]
//             .iter_mut()
//             .enumerate()
//         {
//             *x = enc_utxo[i];
//         }
//         user_account_data.modified_ranges.push(modifying_index);
//     }

//     UserAccount::pack_into_slice(&user_account_data, &mut account.data.borrow_mut());
//     Ok(())
// }

// pub fn close_user_account(
//     account: &AccountInfo,
//     signer: &AccountInfo,
//     rent: Rent,
// ) -> Result<(), ProgramError> {
//     let user_account_data = UserAccount::unpack(&account.data.borrow())?;

//     if user_account_data.owner_pubkey != *signer.key {
//         msg!("Wrong signer.");
//         return Err(ProgramError::InvalidArgument);
//     }

//     //check for rent exemption
//     if !rent.is_exempt(**account.lamports.borrow(), account.data.borrow().len()) {
//         msg!("User account is not active.");
//         return Err(ProgramError::AccountNotRentExempt);
//     }
//     //close account by draining lamports
//     let dest_starting_lamports = signer.lamports();
//     **signer.lamports.borrow_mut() = dest_starting_lamports
//         .checked_add(account.lamports())
//         .ok_or(ProgramError::InvalidAccountData)?;
//     **account.lamports.borrow_mut() = 0;
//     Ok(())
// }
