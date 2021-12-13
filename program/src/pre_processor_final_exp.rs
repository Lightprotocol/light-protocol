
use crate::ranges_part_2::*;
use crate::state_final_exp::{FinalExpBytes, INSTRUCTION_ORDER_VERIFIER_PART_2};
use crate::state_merkle_tree;


use solana_program::{
    msg,
    log::sol_log_compute_units,
    account_info::{next_account_info, AccountInfo},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
};

use crate::instructions_final_exponentiation::check_and_insert_nullifier;

use crate::processor_final_exp::_process_instruction_final_exp;

pub fn _pre_process_instruction_final_exp(program_id: &Pubkey, accounts: &[AccountInfo], _instruction_data: &[u8]) -> Result<(),ProgramError>{
    let account = &mut accounts.iter();
    let signing_account = next_account_info(account)?;
    let storage_acc = next_account_info(account)?; // always the storage account no matter which part (1,2, merkletree)

    sol_log_compute_units();
    let mut storage_acc_data = FinalExpBytes::unpack(&storage_acc.data.borrow())?;
    msg!("index {}", storage_acc_data.current_instruction_index);
    sol_log_compute_units();
    //check for signer
    //assert_eq!(*signing_account.key, solana_program::pubkey::Pubkey::new(&storage_acc_data.signing_address[..]));

    if storage_acc_data.current_instruction_index == 1821 {
        let executed_miller_loop = true;
        storage_acc_data.current_instruction_index = 0;
    }
    //check that instruction order is called correctly
    //msg!("PT 2 lib.rs - Instruction {} {}", INSTRUCTION_ORDER_VERIFIER_PART_2[storage_acc_data.current_instruction_index], storage_acc_data.current_instruction_index);

    //assert_eq!(_instruction_data[8],  INSTRUCTION_ORDER_VERIFIER_PART_2[storage_acc_data.current_instruction_index]);

    /*

    * disabled for testing

    *assert_eq!(*signing_account.key, solana_program::pubkey::Pubkey::new(&storage_acc_data.signing_address), "Invalid sender");
    */


    //init instruction for testing
    // if _instruction_data[0] == 240{

    //     msg!("initatited acc {}", storage_acc_data.f1_r_range_s.len());
    //     let data = get_miller_loop_bytes();
    //     for i in 0..576 {
    //         storage_acc_data.f1_r_range_s[i] = data[i].clone();
    //         storage_acc_data.f_f2_range_s[i] = data[i].clone();
    //     }
    //     storage_acc_data.changed_variables[0]=true;
    //     storage_acc_data.changed_variables[1]=true;

    //     msg!("0 {}", storage_acc_data.f1_r_range_s[0] );
    //     msg!("566 {}", storage_acc_data.f1_r_range_s[566] );

    /*
    if  INSTRUCTION_ORDER_VERIFIER_PART_2[storage_acc_data.current_instruction_index] == 103 {
        let account_from = next_account_info(account)?;
        let account_to = next_account_info(account)?;
        //let f_f2_range_s = parse_f_from_bytes_new(&storage_acc_data.f_f2_range_s);
        // msg!("f_f2_range_s: {:?} ", f_f2_range_s);
        assert_eq!(storage_acc_data.found_nullifier,2, "nullifier_hash already exists");
        //let merkletree_acc_bytes: [u8;32] = [251, 30, 194, 174, 168, 85, 13, 188, 134, 0, 17, 157, 187, 32, 113, 104, 134, 138, 82, 128, 95, 206, 76, 34, 177, 163, 246, 27, 109, 207, 2, 85];
        //check that withdraw is from merkletree account
        assert_eq!(*account_from.key, solana_program::pubkey::Pubkey::new(&state_merkle_tree::MERKLE_TREE_ACC_BYTES[..]));

        //check that withdraw is to right account
        assert_eq!(*account_to.key, solana_program::pubkey::Pubkey::new(&storage_acc_data.to_address[..]));
        //failing test
        //assert_eq!(*account_to.key, solana_program::pubkey::Pubkey::new(&merkletree_acc_bytes[..]));

         verify_result_and_withdraw(&storage_acc_data.f1_r_range_s,&account_from, &account_to);
    }
    //check nullifier

    else if INSTRUCTION_ORDER_VERIFIER_PART_2[storage_acc_data.current_instruction_index] == 121 {
        //msg!("starting nullifier check {:?}", account);
        let account_merkle_tree = next_account_info(account)?;
        msg!("starting nullifier check0 {}", storage_acc_data.found_nullifier);
        check_nullifier_in_range_0(&account_merkle_tree, &storage_acc_data.nullifer ,&mut storage_acc_data.found_nullifier);
        storage_acc_data.changed_variables[14] = true;
        sol_log_compute_units();

        //return Ok(());
    } else if INSTRUCTION_ORDER_VERIFIER_PART_2[storage_acc_data.current_instruction_index] == 122 {
        //msg!("starting nullifier check {:?}", account);
        let account_merkle_tree = next_account_info(account)?;
        msg!("starting nullifier check1 {}", storage_acc_data.found_nullifier);
        check_nullifier_in_range_1(&account_merkle_tree, &storage_acc_data.nullifer ,&mut storage_acc_data.found_nullifier);
        storage_acc_data.changed_variables[14] = true;
        //return Ok(());
    } else if INSTRUCTION_ORDER_VERIFIER_PART_2[storage_acc_data.current_instruction_index] == 123 {
        //msg!("starting nullifier check {:?}", account);
        let account_merkle_tree = next_account_info(account)?;
        msg!("starting nullifier check2 {}", storage_acc_data.found_nullifier);
        check_nullifier_in_range_2(&account_merkle_tree, &storage_acc_data.nullifer ,&mut storage_acc_data.found_nullifier);
        storage_acc_data.changed_variables[14] = true;
        //return Ok(());
    } else if INSTRUCTION_ORDER_VERIFIER_PART_2[storage_acc_data.current_instruction_index] == 124 {
        //msg!("starting nullifier check {:?}", account);
        let account_merkle_tree = next_account_info(account)?;
        msg!("starting nullifier check3 {}", storage_acc_data.found_nullifier);
        check_nullifier_in_range_3(&account_merkle_tree, &storage_acc_data.nullifer ,&mut storage_acc_data.found_nullifier);
        storage_acc_data.changed_variables[14] = true;
        //return Ok(());



    } else {*/

    // if storage_acc_data.current_instruction_index > 0 {
    //     let x = [0, 121, 122];
    //     let instruction_id = storage_acc_data.current_instruction_index.clone();
    //
    //     _process_instruction_final_exp(  &mut storage_acc_data, x[instruction_id]);
    // }
    // else
    if storage_acc_data.current_instruction_index == 700 {
        msg!("checking nullifier");

        let nullifier_account = next_account_info(account)?;
        check_and_insert_nullifier(program_id, nullifier_account, &_instruction_data[10..42]);
    } else {
        let instruction_id = INSTRUCTION_ORDER_VERIFIER_PART_2[storage_acc_data.current_instruction_index.clone()];
        msg!("verify instruction : {}", instruction_id);
        _process_instruction_final_exp(  &mut storage_acc_data, instruction_id);
    }

    storage_acc_data.current_instruction_index +=1;
    sol_log_compute_units();

    FinalExpBytes::pack_into_slice(&storage_acc_data, &mut storage_acc.data.borrow_mut());
    sol_log_compute_units();
    //msg!("packed: {:?}", storage_acc_data.changed_variables);

    Ok(())
}
