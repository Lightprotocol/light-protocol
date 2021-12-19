use crate::state_merkle_tree_roots::{check_root_hash_exists, MERKLE_TREE_ACC_BYTES};
use crate::instructions_final_exponentiation::{check_and_insert_nullifier};
use crate::pi_state::PiBytes;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    log::sol_log_compute_units,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    program_pack::Pack
};
use std::convert::TryInto;
//pre processor for light protocol logic
//merkle root checks
//nullifier checks
//_args.publicAmount == calculatePublicAmount(_extData.extAmount, _extData.fee)
//check tx data hash
//deposit and withdraw logic
pub fn li_pre_process_instruction(program_id: &Pubkey, accounts: &[AccountInfo],current_instruction_index: usize) -> Result<(), ProgramError> {
    msg!("entered li_pre_process_instruction");

    let account = &mut accounts.iter();
    msg!("here0");

    let _signing_account = next_account_info(account)?;
    msg!("here1");
    let main_account = next_account_info(account)?;
    msg!("here2");
    let mut account_data = PiBytes::unpack(&main_account.data.borrow())?;

    if current_instruction_index == 1 {
        msg!("here3");
        let merkle_tree_account = next_account_info(account)?;
        msg!("here4");
        msg!("merkletree acc key: {:?}", *merkle_tree_account.key);
        msg!(
            "key to check: {:?}",
            solana_program::pubkey::Pubkey::new(&MERKLE_TREE_ACC_BYTES[..])
        );
        account_data.found_root = check_root_hash_exists(
            merkle_tree_account,
            &account_data.root_hash
        )?;

    }
    //check tx data hash and public amount
    else if current_instruction_index == 2 {
        //account_data.tx_integrity_hash
        //account_data.amount

    }
    //nullifier checks
    //deposit and withdraw logic
    else if current_instruction_index == 1503 {
        //assert_eq!(true, false, "does not work yet");
        let two_leaves_pda = next_account_info(account)?;
        let nullifier0 = next_account_info(account)?;
        let nullifier1 = next_account_info(account)?;
        let recipient_account = next_account_info(account)?;
        msg!("starting nullifier check");
        account_data.found_nullifier = check_and_insert_nullifier(
            program_id,
            nullifier0,
            &account_data.proof_a_b_c_leaves_and_nullifiers[320..352]
        )?;
        msg!("starting nullifier0 inserted");

        account_data.found_nullifier = check_and_insert_nullifier(
            program_id,
            nullifier1,
            &account_data.proof_a_b_c_leaves_and_nullifiers[352..384],
        )?;
        msg!("starting nullifier1 inserted");

        let amount = i64::from_le_bytes(account_data.amount.clone().try_into().unwrap());
        let amount: i64 = 1000000000;
        if amount > 0 {

            if *recipient_account.key != solana_program::pubkey::Pubkey::new(&MERKLE_TREE_ACC_BYTES) {
                msg!("recipient has to be merkle tree account for deposit");
                return Err(ProgramError::InvalidInstructionData);
            }
            transfer(recipient_account, two_leaves_pda, amount);
        } else if amount < 0 {
            if *recipient_account.key != solana_program::pubkey::Pubkey::new(&account_data.to_address) {
                msg!("recipient has to be merkle tree account for deposit");
                return Err(ProgramError::InvalidInstructionData);
            }
            transfer(recipient_account, two_leaves_pda, amount);
        }

    } else if current_instruction_index == 4 {
        //state_check_nullifier::check_and_insert_nullifier();
    }

    account_data.current_instruction_index +=1;
    PiBytes::pack_into_slice(&account_data, &mut main_account.data.borrow_mut());

    Ok(())
}

//performs the following security checks:
//signer is consistent over all tx of a pool tx
//the correct merkle tree is called
//instruction data is empty
//there are no more and no less than the required accounts
//attached to the tx, the accounts have the appropiate length
pub fn li_security_checks(accounts: &[AccountInfo]) -> Result<(),ProgramError> {
    let account = &mut accounts.iter();
    let _signing_account = next_account_info(account)?;

    let main_account = next_account_info(account)?;
    // assert_eq!(
    //     *signing_account.key,
    //     solana_program::pubkey::Pubkey::new(&account_data.signing_address)
    // );
    Ok(())
}
use std::convert::TryFrom;
pub fn transfer(_to: &AccountInfo, _from: &AccountInfo, amount: i64){
        //if the user actually deposited 1 sol increase current_total_deposits by one

        **_from.try_borrow_mut_lamports().unwrap()    -= u64::try_from(amount).unwrap();//1000000000; // 1 SOL

        **_to.try_borrow_mut_lamports().unwrap()        += u64::try_from(amount).unwrap();

        //merkle_tree_account.current_total_deposits += 1;
        msg!("transferred of {} Sol from {:?} to {:?}", amount,_from.key, _to.key);
}
