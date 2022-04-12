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
