use crate::user_account::state::{UserAccount, SIZE_UTXO};
use solana_program::{
    account_info::AccountInfo, msg, program_error::ProgramError, program_pack::Pack,
    pubkey::Pubkey, sysvar::rent::Rent,
};
use std::convert::TryInto;

pub fn initialize_user_account(
    account: &AccountInfo,
    pubkey_signer: Pubkey,
) -> Result<(), ProgramError> {
    //check for rent exemption
    let rent = Rent::free();
    if rent.is_exempt(**account.lamports.borrow(), account.data.borrow().len()) != true {
        msg!("user account is not rentexempt");
        return Err(ProgramError::InvalidInstructionData);
    }

    //initialize
    msg!("here1");
    let mut user_account_data = UserAccount::unpack(&account.data.borrow())?;
    msg!("here2");
    user_account_data.owner_pubkey = pubkey_signer.clone();
    msg!("here3");
    UserAccount::pack_into_slice(&user_account_data, &mut account.data.borrow_mut());
    msg!("here4");
    Ok(())
}

pub fn modify_user_account(
    account: &AccountInfo,
    signer: Pubkey,
    data: &[u8],
) -> Result<(), ProgramError> {
    let mut user_account_data = UserAccount::unpack(&account.data.borrow())?;

    if user_account_data.owner_pubkey != signer {
        msg!("wrong signer");
        return Err(ProgramError::InvalidArgument);
    }
    for y in data.chunks(8 + SIZE_UTXO as usize) {
        //first 8 bytes are index
        let modifying_index = usize::from_be_bytes(y[0..8].try_into().unwrap());
        //last 64 bytes are the utxo
        let enc_utxo = &y[8..SIZE_UTXO as usize + 8];
        for (i, x) in user_account_data.enc_utxos[modifying_index * SIZE_UTXO as usize
            ..modifying_index * SIZE_UTXO as usize + SIZE_UTXO as usize]
            .iter_mut()
            .enumerate()
        {
            *x = enc_utxo[i];
        }
        user_account_data.modified_ranges.push(modifying_index);
    }

    UserAccount::pack_into_slice(&user_account_data, &mut account.data.borrow_mut());
    Ok(())
}
