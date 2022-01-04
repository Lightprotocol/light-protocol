use crate::user_account::state::{UserAccount, SIZE_UTXO};
use solana_program::{
    account_info::AccountInfo,
    //log::sol_log_compute_units,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    program_pack::Pack,
    sysvar::rent::Rent
};
use std::convert::TryInto;

pub fn initialize_user_account(account: &AccountInfo, pubkey_signer: Pubkey) -> Result<(),ProgramError> {

    //check for rent exemption
    let rent = Rent::free();
    if rent.is_exempt(**account.lamports.borrow(), 2) != true {
        msg!("user account is not rentexempt");
        return Err(ProgramError::InvalidInstructionData);
    }

    //initialize
    let mut user_account_data = UserAccount::unpack(&account.data.borrow())?;
    user_account_data.owner_pubkey = pubkey_signer.clone();
    UserAccount::pack_into_slice(&user_account_data, &mut account.data.borrow_mut());
    Ok(())
}

pub fn modify_user_account(account: &AccountInfo, signer: Pubkey, data: &[u8]) -> Result<(),ProgramError> {
    let mut user_account_data = UserAccount::unpack(&account.data.borrow())?;

    // data.chunks(8 + SIZE_UTXO as usize).map(|x| {
    //     //first 8 bytes are index
    //     let modifying_index = usize::from_le_bytes(x[0..8].try_into().unwrap());
    //     msg!("user account modify here2 {:?}", x);
    //     //last 64 bytes are the utxo
    //     let enc_utxo = &x[8..SIZE_UTXO as usize];
    //     msg!("user account modify here3 {:?}", enc_utxo);
    //     user_account_data.enc_utxos[modifying_index*SIZE_UTXO as usize..modifying_index*SIZE_UTXO as usize + SIZE_UTXO as usize].iter_mut().enumerate().map( |(i, x)| {
    //         *x= enc_utxo[i];
    //
    //     });
    //     user_account_data.modified_ranges.push(modifying_index);
    // });
    if user_account_data.owner_pubkey != signer {
        msg!("wrong signer");
        return Err(ProgramError::InvalidArgument);
    }
    for x in data.chunks(8 + SIZE_UTXO as usize) {
        //first 8 bytes are index
        let modifying_index = usize::from_le_bytes(x[0..8].try_into().unwrap());
        //last 64 bytes are the utxo
        let enc_utxo = &x[8..SIZE_UTXO as usize + 8];
        for (i, x) in user_account_data.enc_utxos[modifying_index*SIZE_UTXO as usize..modifying_index*SIZE_UTXO as usize + SIZE_UTXO as usize].iter_mut().enumerate() {
            *x= enc_utxo[i];
            //msg!("i {}, x {}", i, x);
        }
        user_account_data.modified_ranges.push(modifying_index);
    }

    UserAccount::pack_into_slice(&user_account_data, &mut account.data.borrow_mut());
    Ok(())
}
