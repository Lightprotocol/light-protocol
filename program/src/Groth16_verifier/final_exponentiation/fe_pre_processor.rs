use crate::Groth16_verifier::final_exponentiation::{
    fe_ranges::*,
    fe_state::FinalExpBytes,
    fe_processor::_process_instruction_final_exp,

};

use crate::IX_ORDER;

use solana_program::{
    msg,
    log::sol_log_compute_units,
    account_info::{next_account_info, AccountInfo},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
};

pub fn _pre_process_instruction_final_exp(program_id: &Pubkey, accounts: &[AccountInfo], _instruction_data: &[u8]) -> Result<(),ProgramError>{
    let account = &mut accounts.iter();
    let signing_account = next_account_info(account)?;
    let storage_acc = next_account_info(account)?; // always the storage account no matter which part (1,2, merkletree)

    sol_log_compute_units();
    let mut storage_acc_data = FinalExpBytes::unpack(&storage_acc.data.borrow())?;
    msg!("index {}", storage_acc_data.current_instruction_index);
    sol_log_compute_units();

    let instruction_id = IX_ORDER[storage_acc_data.current_instruction_index.clone()];
    msg!("verify instruction : {}", instruction_id);
    _process_instruction_final_exp(  &mut storage_acc_data, instruction_id);


    storage_acc_data.current_instruction_index +=1;
    sol_log_compute_units();

    FinalExpBytes::pack_into_slice(&storage_acc_data, &mut storage_acc.data.borrow_mut());
    sol_log_compute_units();
    //msg!("packed: {:?}", storage_acc_data.changed_variables);

    Ok(())
}
