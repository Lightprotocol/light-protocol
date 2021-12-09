
use crate::ranges_part_2::*;
use crate::state_final_exp::{FinalExpBytes, INSTRUCTION_ORDER_VERIFIER_PART_2};
use crate::state_merkle_tree;


use solana_program::{
    msg,
    log::sol_log_compute_units,
    account_info::{next_account_info, AccountInfo},
    program_error::ProgramError,
    program_pack::Pack,
};
use crate::utils::verify_result_and_withdraw;
use crate::state_check_nullifier::{
    check_nullifier_in_range_0,
    check_nullifier_in_range_1,
    check_nullifier_in_range_2,
    check_nullifier_in_range_3,
};

use crate::processor_final_exp::_process_instruction_final_exp;

pub fn _pre_process_instruction_final_exp(_instruction_data: &[u8], accounts: &[AccountInfo]) -> Result<(),ProgramError>{
    let account = &mut accounts.iter();
    let signing_account = next_account_info(account)?;
    let storage_acc = next_account_info(account)?; // always the storage account no matter which part (1,2, merkletree)
    sol_log_compute_units();
    let mut storage_acc_data = FinalExpBytes::unpack(&storage_acc.data.borrow())?;
    sol_log_compute_units();
    //check for signer
    //assert_eq!(*signing_account.key, solana_program::pubkey::Pubkey::new(&storage_acc_data.signing_address[..]));

    if storage_acc_data.current_instruction_index == 1821 {
        let executed_miller_loop = true;
        storage_acc_data.current_instruction_index = 0;
    }
    //check that instruction order is called correctly
    msg!("PT 2 lib.rs - Instruction {} {}", INSTRUCTION_ORDER_VERIFIER_PART_2[storage_acc_data.current_instruction_index], storage_acc_data.current_instruction_index);

    assert_eq!(_instruction_data[8],  INSTRUCTION_ORDER_VERIFIER_PART_2[storage_acc_data.current_instruction_index]);

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
        msg!("verify instruction ");
        let instruction_id = INSTRUCTION_ORDER_VERIFIER_PART_2[storage_acc_data.current_instruction_index.clone()];
        _process_instruction_final_exp(  &mut storage_acc_data, instruction_id);
    //}

    storage_acc_data.current_instruction_index +=1;
    sol_log_compute_units();

    FinalExpBytes::pack_into_slice(&storage_acc_data, &mut storage_acc.data.borrow_mut());
    sol_log_compute_units();
    msg!("packed: {:?}", storage_acc_data.changed_variables);

    Ok(())
}

/*
pub fn _process_instruction_part_2(id: u8, account_struct: &mut FinalExpBytes) {
    msg!("PIP2 - calling instruction {}", id);

    if id == 0 {
            msg!("PIP2 - calling instruction data {:?}", account_struct.f1_r_range_s);

            conjugate_wrapper(&mut account_struct.f1_r_range_s);
            account_struct.changed_variables[f1_r_range_iter] = true;

        } else if id == 1 {
            custom_f_inverse_1(&account_struct.f_f2_range_s, &mut account_struct.cubic_range_1_s);
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 2 {
            custom_f_inverse_2(&account_struct.f_f2_range_s,&mut account_struct.cubic_range_0_s, &account_struct.cubic_range_1_s);
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 3 {
            custom_cubic_inverse_1(
                &account_struct.cubic_range_0_s,
                &mut account_struct.quad_range_0_s,
                &mut account_struct.quad_range_1_s,
                &mut account_struct.quad_range_2_s,
                &mut account_struct.quad_range_3_s
            );
            account_struct.changed_variables[quad_range_0_iter] = true;
            account_struct.changed_variables[quad_range_1_iter] = true;
            account_struct.changed_variables[quad_range_2_iter] = true;
            account_struct.changed_variables[quad_range_3_iter] = true;

        } else if id == 4 {
            custom_quadratic_fp384_inverse_1(
                &account_struct.quad_range_3_s,
                &mut account_struct.fp384_range_s
            );
            account_struct.changed_variables[fp384_range_iter] = true;

        } else if id == 5 {
            custom_quadratic_fp384_inverse_2(
                &mut account_struct.quad_range_3_s,
                & account_struct.fp384_range_s,
            );
            account_struct.changed_variables[quad_range_3_iter] = true;

        } else if id == 6 {
            custom_cubic_inverse_2(
            &mut account_struct.cubic_range_0_s,
            & account_struct.quad_range_0_s,
            & account_struct.quad_range_1_s,
            & account_struct.quad_range_2_s,
            & account_struct.quad_range_3_s
        );
        account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 7 {
            custom_f_inverse_3(
                &mut account_struct.cubic_range_1_s,
                &account_struct.cubic_range_0_s,
                &account_struct.f_f2_range_s,
            );
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 8 {
            //mul_assign_1(&mut account_struct[..], f1_r_cubic_0_range, f_f2_cubic_0_range, cubic_range_0);
            mul_assign_1(
                &account_struct.f1_r_range_s, f_cubic_0_range,
                &account_struct.f_f2_range_s, f_cubic_0_range,
                &mut account_struct.cubic_range_0_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 9 {
            //mul_assign_2(&mut account_struct[..], f1_r_cubic_1_range, f_f2_cubic_1_range, cubic_range_1);
            mul_assign_2(
                &account_struct.f1_r_range_s, f_cubic_1_range,
                &account_struct.f_f2_range_s, f_cubic_1_range,
                &mut account_struct.cubic_range_1_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 10 {
            //mul_assign_3(&mut account_struct[..], f1_r_range);

            mul_assign_3(
                &mut account_struct.f1_r_range_s
            );
            account_struct.changed_variables[f1_r_range_iter] = true;

        } else if id == 11 {
            //mul_assign_4(&mut account_struct[..], f1_r_cubic_1_range, f_f2_range);
            mul_assign_4_1(
                &account_struct.f_f2_range_s,
                &mut account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 104 {
            //mul_assign_4(&mut account_struct[..], f1_r_cubic_1_range, f_f2_range);
            mul_assign_4_2(
                &mut account_struct.f1_r_range_s,
                f_cubic_1_range,
                &account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[f1_r_range_iter] = true;

        }  else if id == 12 {
            //mul_assign_5(&mut account_struct[..], f1_r_range, cubic_range_0, cubic_range_1);
            mul_assign_5(
                &mut account_struct.f1_r_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[f1_r_range_iter] = true;

        } else if id == 13 {
            //assign_range_x_to_range_y(account_struct, f1_r_range, f_f2_range);
            account_struct.f_f2_range_s = account_struct.f1_r_range_s.clone();
            account_struct.changed_variables[f_f2_range_iter] = true;


        } else if id == 14 {
            custom_frobenius_map_2_1(&mut account_struct.f1_r_range_s);
            account_struct.changed_variables[f1_r_range_iter] = true;

        } else if id == 15 {
            custom_frobenius_map_2_2(&mut account_struct.f1_r_range_s);
            account_struct.changed_variables[f1_r_range_iter] = true;
            //works
        } else if id == 16 {
            custom_cyclotomic_square(&account_struct.f1_r_range_s, &mut account_struct.y0_range_s);
            account_struct.changed_variables[y0_range_iter] = true;
            //works
        } else if id == 17 {
            //assign_range_x_to_range_y(account_struct, f1_r_range, i_range);
            account_struct.i_range_s = account_struct.f1_r_range_s.clone();
            account_struct.changed_variables[i_range_iter] = true;

        } else if id == 18 {
            conjugate_wrapper(&mut account_struct.i_range_s);
            account_struct.changed_variables[i_range_iter] = true;

        } else if id == 19 {
            //assign_range_x_to_range_y(account_struct, f1_r_range, y1_range);
            account_struct.y1_range_s = account_struct.f1_r_range_s.clone();
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 20 {
            //custom_square_in_place_instruction_else_1(account_struct, y1_range, cubic_range_0, cubic_range_1, cubic_range_2);
            custom_square_in_place_instruction_else_1(
                & account_struct.y1_range_s,
                &mut account_struct.cubic_range_0_s,
                //&mut account_struct.cubic_range_1_s,
                &mut account_struct.cubic_range_2_s
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 105 {
            //custom_square_in_place_instruction_else_1(account_struct, y1_range, cubic_range_0, cubic_range_1, cubic_range_2);
            custom_square_in_place_instruction_else_1_2(
                & account_struct.y1_range_s,
                &mut account_struct.cubic_range_1_s,
            );
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 21 {
            custom_square_in_place_instruction_else_2(&mut account_struct.cubic_range_0_s, &account_struct.cubic_range_2_s);
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 22 {
            custom_square_in_place_instruction_else_3(
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s,
                &mut account_struct.y1_range_s,
                f_cubic_0_range,
                f_cubic_1_range
            );
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 23 {
            //mul_assign_1(&mut account_struct[..], y1_cubic_0_range, f1_r_cubic_0_range, cubic_range_0);
            mul_assign_1(
                &account_struct.y1_range_s, f_cubic_0_range,
                &account_struct.f1_r_range_s, f_cubic_0_range,
                &mut account_struct.cubic_range_0_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;
        } else if id == 24 {
            //mul_assign_2(&mut account_struct[..], y1_cubic_1_range, f1_r_cubic_1_range, cubic_range_1);
            mul_assign_2(
                &account_struct.y1_range_s, f_cubic_1_range,
                &account_struct.f1_r_range_s, f_cubic_1_range,
                &mut account_struct.cubic_range_1_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 25 {
            //mul_assign_3(&mut account_struct[..], y1_range);
            mul_assign_3(
                &mut account_struct.y1_range_s
            );
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 26 {
            //mul_assign_4(&mut account_struct[..], y1_cubic_1_range, f1_r_range);
            mul_assign_4_1(
                &account_struct.f1_r_range_s,
                &mut account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 106 {
            //mul_assign_4(&mut account_struct[..], y1_cubic_1_range, f1_r_range);
            mul_assign_4_2(
                &mut account_struct.y1_range_s,
                f_cubic_1_range,
                &account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 27 {
            //mul_assign_5(&mut account_struct[..], y1_range, cubic_range_0, cubic_range_1);
            mul_assign_5(
                &mut account_struct.y1_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 28 {
            //mul_assign_1(&mut account_struct[..], y1_cubic_0_range, i_cubic_0_range, cubic_range_0);
            mul_assign_1(
                &account_struct.y1_range_s, f_cubic_0_range,
                &account_struct.i_range_s, f_cubic_0_range,
                &mut account_struct.cubic_range_0_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 29 {
            //mul_assign_2(&mut account_struct[..], y1_cubic_1_range, i_cubic_1_range, cubic_range_1);
            mul_assign_2(
                &account_struct.y1_range_s, f_cubic_1_range,
                &account_struct.i_range_s, f_cubic_1_range,
                &mut account_struct.cubic_range_1_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 30 {
            //mul_assign_3(&mut account_struct[..], y1_range);
            mul_assign_3(
                &mut account_struct.y1_range_s
            );
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 31 {
            //mul_assign_4(&mut account_struct[..], y1_cubic_1_range, i_range);
            mul_assign_4_1(
                &account_struct.i_range_s,
                &mut account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 107 {
            //mul_assign_4(&mut account_struct[..], y1_cubic_1_range, i_range);
            mul_assign_4_2(
                &mut account_struct.y1_range_s,
                f_cubic_1_range,
                &account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[y1_range_iter] = true;

        }  else if id == 32 {
            //mul_assign_5(&mut account_struct[..], y1_range, cubic_range_0, cubic_range_1);
            mul_assign_5(
                &mut account_struct.y1_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[y1_range_iter] = true;


        } else if id == 33 {
            conjugate_wrapper(&mut account_struct.y1_range_s);
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 34 {
            //assign_range_x_to_range_y(&mut account_struct[..], f1_r_range, y2_range);
            account_struct.y2_range_s = account_struct.f1_r_range_s.clone();
            account_struct.changed_variables[y2_range_iter] = true;


        } else if id == 35 {
            conjugate_wrapper(&mut account_struct.y2_range_s);
            account_struct.changed_variables[y2_range_iter] = true;
        } else if id == 36 {
            //mul_assign_1(&mut account_struct[..], y1_cubic_0_range, y2_cubic_0_range, cubic_range_0);
            mul_assign_1(
                &account_struct.y1_range_s, f_cubic_0_range,
                &account_struct.y2_range_s, f_cubic_0_range,
                &mut account_struct.cubic_range_0_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 37 {
            //mul_assign_2(&mut account_struct[..], y1_cubic_1_range, y2_cubic_1_range, cubic_range_1);
            mul_assign_2(
                &account_struct.y1_range_s, f_cubic_1_range,
                &account_struct.y2_range_s, f_cubic_1_range,
                &mut account_struct.cubic_range_1_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 38 {
            //mul_assign_3(&mut account_struct[..], y1_range);
            mul_assign_3(
                &mut account_struct.y1_range_s
            );
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 39 {
            //mul_assign_4(&mut account_struct[..], y1_cubic_1_range, y2_range);
            mul_assign_4_1(
                &account_struct.y2_range_s,
                &mut account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 108 {
            //mul_assign_4(&mut account_struct[..], y1_cubic_1_range, y2_range);
            mul_assign_4_2(
                &mut account_struct.y1_range_s,
                f_cubic_1_range,
                &account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 40 {
            //mul_assign_5(&mut account_struct[..], y1_range, cubic_range_0, cubic_range_1);
            mul_assign_5(
                &mut account_struct.y1_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 41 {
            //assign_range_x_to_range_y(account_struct, y1_range, i_range);
            account_struct.i_range_s = account_struct.y1_range_s.clone();
            account_struct.changed_variables[i_range_iter] = true;

        } else if id == 42 {
            //assign_range_x_to_range_y(account_struct, y1_range, y2_range);
            account_struct.y2_range_s = account_struct.y1_range_s.clone();
            account_struct.changed_variables[y2_range_iter] = true;

        }  else if id == 43 {
            //custom_square_in_place_instruction_else_1(account_struct, y2_range, cubic_range_0, cubic_range_1, cubic_range_2);
            custom_square_in_place_instruction_else_1(
                & account_struct.y2_range_s,
                &mut account_struct.cubic_range_0_s,
                //&mut account_struct.cubic_range_1_s,
                &mut account_struct.cubic_range_2_s
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 109 {
            //custom_square_in_place_instruction_else_1(account_struct, y1_range, cubic_range_0, cubic_range_1, cubic_range_2);
            custom_square_in_place_instruction_else_1_2(
                & account_struct.y2_range_s,
                &mut account_struct.cubic_range_1_s,
            );
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 44 {
            //custom_square_in_place_instruction_else_2(&mut account_struct.cubic_range_0_s, &account_struct.cubic_range_2_s);
            custom_square_in_place_instruction_else_2(
                &mut account_struct.cubic_range_0_s,
                &account_struct.cubic_range_2_s
            );

            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 45 {
            //custom_square_in_place_instruction_else_3(account_struct, cubic_range_0, cubic_range_1, y2_cubic_0_range, y2_cubic_1_range);
            custom_square_in_place_instruction_else_3(
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s,
                &mut account_struct.y2_range_s,
                f_cubic_0_range,
                f_cubic_1_range
            );
            account_struct.changed_variables[y2_range_iter] = true;
        }  else if id == 46 {
            //mul_assign_1(&mut account_struct[..], y2_cubic_0_range, y1_cubic_0_range, cubic_range_0);
            mul_assign_1(
                &account_struct.y2_range_s, f_cubic_0_range,
                &account_struct.y1_range_s, f_cubic_0_range,
                &mut account_struct.cubic_range_0_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 47 {
            //mul_assign_2(&mut account_struct[..], y2_cubic_1_range, y1_cubic_1_range, cubic_range_1);
            mul_assign_2(
                &account_struct.y2_range_s, f_cubic_1_range,
                &account_struct.y1_range_s, f_cubic_1_range,
                &mut account_struct.cubic_range_1_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 48 {
            //mul_assign_3(&mut account_struct[..], y2_range);
            mul_assign_3(
                &mut account_struct.y2_range_s
            );
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 49 {
            //mul_assign_4(&mut account_struct[..], y2_cubic_1_range, y1_range);
            mul_assign_4_1(
                &account_struct.y1_range_s,
                &mut account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 110 {
            //mul_assign_4(&mut account_struct[..], y2_cubic_1_range, y1_range);
            mul_assign_4_2(
                &mut account_struct.y2_range_s,
                f_cubic_1_range,
                &account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[y2_range_iter] = true;

        }  else if id == 50 {
            //mul_assign_5(&mut account_struct[..], y2_range, cubic_range_0, cubic_range_1);
            mul_assign_5(
                &mut account_struct.y2_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 51 {
            //mul_assign_1(&mut account_struct[..], y2_cubic_0_range, i_cubic_0_range, cubic_range_0);
            mul_assign_1(
                &account_struct.y2_range_s, f_cubic_0_range,
                &account_struct.i_range_s, f_cubic_0_range,
                &mut account_struct.cubic_range_0_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 52 {
            //mul_assign_2(&mut account_struct[..], y2_cubic_1_range, i_cubic_1_range, cubic_range_1);
            mul_assign_2(
                &account_struct.y2_range_s, f_cubic_1_range,
                &account_struct.i_range_s, f_cubic_1_range,
                &mut account_struct.cubic_range_1_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 53 {
            //mul_assign_3(&mut account_struct[..], y2_range);
            mul_assign_3(
                &mut account_struct.y2_range_s
            );
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 54 {
            //mul_assign_4(&mut account_struct[..], y2_cubic_1_range, i_range);
            mul_assign_4_1(
                &account_struct.i_range_s,
                &mut account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 111 {
            //mul_assign_4(&mut account_struct[..], y2_cubic_1_range, i_range);
            mul_assign_4_2(
                &mut account_struct.y2_range_s,
                f_cubic_1_range,
                &account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 55 {
            //mul_assign_5(&mut account_struct[..], y2_range, cubic_range_0, cubic_range_1);
            mul_assign_5(
                &mut account_struct.y2_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 56 {
            conjugate_wrapper(&mut account_struct.y2_range_s);
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 57 {
            conjugate_wrapper(&mut account_struct.y1_range_s);
            account_struct.changed_variables[y1_range_iter] = true;

        }  else if id == 58 {
            custom_frobenius_map_1_1(&mut account_struct.y1_range_s);
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 59 {
            custom_frobenius_map_1_2(&mut account_struct.y1_range_s);
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 61 {
            //mul_assign_1(&mut account_struct[..], f1_r_cubic_0_range, y0_cubic_0_range, cubic_range_0);
            mul_assign_1(
                &account_struct.f1_r_range_s, f_cubic_0_range,
                &account_struct.y0_range_s, f_cubic_0_range,
                &mut account_struct.cubic_range_0_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 62 {
            //mul_assign_2(&mut account_struct[..], f1_r_cubic_1_range, y0_cubic_1_range, cubic_range_1);
            mul_assign_2(
                &account_struct.f1_r_range_s, f_cubic_1_range,
                &account_struct.y0_range_s, f_cubic_1_range,
                &mut account_struct.cubic_range_1_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 63 {
            //mul_assign_3(&mut account_struct[..], f1_r_range);
            mul_assign_3(
                &mut account_struct.f1_r_range_s
            );
            account_struct.changed_variables[f1_r_range_iter] = true;

        } else if id == 64 {
            //mul_assign_4(&mut account_struct[..], f1_r_cubic_1_range, y0_range);
            mul_assign_4_1(
                &account_struct.y0_range_s,
                &mut account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 112 {
            //mul_assign_4(&mut account_struct[..], f1_r_cubic_1_range, y0_range);
            mul_assign_4_2(
                &mut account_struct.f1_r_range_s,
                f_cubic_1_range,
                &account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[f1_r_range_iter] = true;

        } else if id == 65 {
            //mul_assign_5(&mut account_struct[..], f1_r_range, cubic_range_0, cubic_range_1);
            mul_assign_5(
                &mut account_struct.f1_r_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[f1_r_range_iter] = true;

        } else if id == 66 {
            //assign_range_x_to_range_y(account_struct, y1_range, y0_range);
            account_struct.y0_range_s = account_struct.y1_range_s.clone();
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 67 {
            //custom_square_in_place_instruction_else_1(account_struct, y0_range, cubic_range_0, cubic_range_1, cubic_range_2);
            custom_square_in_place_instruction_else_1(
                & account_struct.y0_range_s,
                &mut account_struct.cubic_range_0_s,
                //&mut account_struct.cubic_range_1_s,
                &mut account_struct.cubic_range_2_s
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 113 {
            //custom_square_in_place_instruction_else_1(account_struct, y0_range, cubic_range_0, cubic_range_1, cubic_range_2);
            custom_square_in_place_instruction_else_1_2(
                & account_struct.y0_range_s,
                &mut account_struct.cubic_range_1_s,
            );
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 68 {
            //custom_square_in_place_instruction_else_2(account_struct, cubic_range_0, cubic_range_2);
            custom_square_in_place_instruction_else_2(&mut account_struct.cubic_range_0_s, &account_struct.cubic_range_2_s);
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 69 {
            //custom_square_in_place_instruction_else_3(account_struct, cubic_range_0, cubic_range_1, y0_cubic_0_range, y0_cubic_1_range);
            custom_square_in_place_instruction_else_3(
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s,
                &mut account_struct.y0_range_s,
                f_cubic_0_range,
                f_cubic_1_range
            );
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 70 {
            //mul_assign_1(&mut account_struct[..], y0_cubic_0_range, y1_cubic_0_range, cubic_range_0);
            mul_assign_1(
                &account_struct.y0_range_s, f_cubic_0_range,
                &account_struct.y1_range_s, f_cubic_0_range,
                &mut account_struct.cubic_range_0_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 71 {
            //mul_assign_2(&mut account_struct[..], y0_cubic_1_range, y1_cubic_1_range, cubic_range_1);
            mul_assign_2(
                &account_struct.y0_range_s, f_cubic_1_range,
                &account_struct.y1_range_s, f_cubic_1_range,
                &mut account_struct.cubic_range_1_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 72 {
            //mul_assign_3(&mut account_struct[..], y0_range);
            mul_assign_3(
                &mut account_struct.y0_range_s
            );
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 73 {
            //mul_assign_4(&mut account_struct[..], y0_cubic_1_range, y1_range);
            mul_assign_4_1(
                &account_struct.y1_range_s,
                &mut account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 114 {
            //mul_assign_4(&mut account_struct[..], y0_cubic_1_range, y1_range);
            mul_assign_4_2(
                &mut account_struct.y0_range_s,
                f_cubic_1_range,
                &account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 74 {
            //mul_assign_5(&mut account_struct[..], y0_range, cubic_range_0, cubic_range_1);
            mul_assign_5(
                &mut account_struct.y0_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 75 {
            //mul_assign_1(&mut account_struct[..], y0_cubic_0_range, i_cubic_0_range, cubic_range_0);
            mul_assign_1(
                &account_struct.y0_range_s, f_cubic_0_range,
                &account_struct.i_range_s, f_cubic_0_range,
                &mut account_struct.cubic_range_0_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 76 {
            //mul_assign_2(&mut account_struct[..], y0_cubic_1_range, i_cubic_1_range, cubic_range_1);
            mul_assign_2(
                &account_struct.y0_range_s, f_cubic_1_range,
                &account_struct.i_range_s, f_cubic_1_range,
                &mut account_struct.cubic_range_1_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 77 {
            //mul_assign_3(&mut account_struct[..], y0_range);
            mul_assign_3(
                &mut account_struct.y0_range_s
            );
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 78 {
            //mul_assign_4(&mut account_struct[..], y0_cubic_1_range, i_range);
            mul_assign_4_1(
                &account_struct.i_range_s,
                &mut account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 115 {
            //mul_assign_4(&mut account_struct[..], y0_cubic_1_range, i_range);
            mul_assign_4_2(
                &mut account_struct.y0_range_s,
                f_cubic_1_range,
                &account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 79 {
            //mul_assign_5(&mut account_struct[..], y0_range, cubic_range_0, cubic_range_1);
            mul_assign_5(
                &mut account_struct.y0_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 80 {
            conjugate_wrapper(&mut account_struct.y0_range_s);
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 81 {
            //assign_range_x_to_range_y(account_struct, y0_range, i_range);
            account_struct.i_range_s = account_struct.y0_range_s.clone();
            account_struct.changed_variables[i_range_iter] = true;

        } else if id == 82 {
            //assign_range_x_to_range_y(account_struct, y0_range, y2_range);
            account_struct.y2_range_s = account_struct.y0_range_s.clone();
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 83 {
            //mul_assign_1(&mut account_struct[..], y2_cubic_0_range, y0_cubic_0_range, cubic_range_0);
            mul_assign_1(
                &account_struct.y2_range_s, f_cubic_0_range,
                &account_struct.y0_range_s, f_cubic_0_range,
                &mut account_struct.cubic_range_0_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 84 {
            //mul_assign_2(&mut account_struct[..], y2_cubic_1_range, y0_cubic_1_range, cubic_range_1);
            mul_assign_2(
                &account_struct.y2_range_s, f_cubic_1_range,
                &account_struct.y0_range_s, f_cubic_1_range,
                &mut account_struct.cubic_range_1_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 85 {
            //mul_assign_3(&mut account_struct[..], y2_range);
            mul_assign_3(
                &mut account_struct.y2_range_s
            );
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 86 {
            //mul_assign_4(&mut account_struct[..], y2_cubic_1_range, y0_range);
            mul_assign_4_1(
                &account_struct.y0_range_s,
                &mut account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 116 {
            //mul_assign_4(&mut account_struct[..], y2_cubic_1_range, y0_range);
            mul_assign_4_2(
                &mut account_struct.y2_range_s,
                f_cubic_1_range,
                &account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 87 {
            //mul_assign_5(&mut account_struct[..], y2_range, cubic_range_0, cubic_range_1);
            mul_assign_5(
                &mut account_struct.y2_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 88 {
            //assign_range_x_to_range_y(account_struct, y1_range, y0_range);
            account_struct.y0_range_s = account_struct.y1_range_s.clone();
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 89 {
            custom_frobenius_map_2_1(&mut account_struct.y0_range_s);
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 90 {
            custom_frobenius_map_2_2(&mut account_struct.y0_range_s);
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 91 {
            //mul_assign_1(&mut account_struct[..], y1_cubic_0_range, y0_cubic_0_range, cubic_range_0);
            mul_assign_1(
                &account_struct.y1_range_s, f_cubic_0_range,
                &account_struct.y0_range_s, f_cubic_0_range,
                &mut account_struct.cubic_range_0_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_0_iter] = true

        } else if id == 92 {
            //mul_assign_2(&mut account_struct[..], y1_cubic_1_range, y0_cubic_1_range, cubic_range_1);
            mul_assign_2(
                &account_struct.y1_range_s, f_cubic_1_range,
                &account_struct.y0_range_s, f_cubic_1_range,
                &mut account_struct.cubic_range_1_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 93 {
            //mul_assign_3(&mut account_struct[..], y1_range);
            mul_assign_3(
                &mut account_struct.y1_range_s
            );
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 94 {
            //mul_assign_4(&mut account_struct[..], y1_cubic_1_range, y0_range);
            mul_assign_4_1(
                &account_struct.y0_range_s,
                &mut account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 117 {
            //mul_assign_4(&mut account_struct[..], y1_cubic_1_range, y0_range);
            mul_assign_4_2(
                &mut account_struct.y1_range_s,
                f_cubic_1_range,
                &account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[y1_range_iter] = true;
        } else if id == 95 {
            //mul_assign_5(&mut account_struct[..], y1_range, cubic_range_0, cubic_range_1);
            mul_assign_5(
                &mut account_struct.y1_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 96 {
            //mul_assign_1(&mut account_struct[..], f1_r_cubic_0_range, y1_cubic_0_range, cubic_range_0);
            mul_assign_1(
                &account_struct.f1_r_range_s, f_cubic_0_range,
                &account_struct.y1_range_s, f_cubic_0_range,
                &mut account_struct.cubic_range_0_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 97 {
            //mul_assign_2(&mut account_struct[..], f1_r_cubic_1_range, y1_cubic_1_range, cubic_range_1);
            mul_assign_2(
                &account_struct.f1_r_range_s, f_cubic_1_range,
                &account_struct.y1_range_s, f_cubic_1_range,
                &mut account_struct.cubic_range_1_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 98 {
            //mul_assign_3(&mut account_struct[..], f1_r_range);
            mul_assign_3(
                &mut account_struct.f1_r_range_s
            );
            account_struct.changed_variables[f1_r_range_iter] = true;

        } else if id == 99 {
            //mul_assign_4(&mut account_struct[..], f1_r_cubic_1_range, y1_range);
            mul_assign_4_1(
                &account_struct.y1_range_s,
                &mut account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 118 {
            //mul_assign_4(&mut account_struct[..], f1_r_cubic_1_range, y1_range);
            mul_assign_4_2(
                &mut account_struct.f1_r_range_s,
                f_cubic_1_range,
                &account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[f1_r_range_iter] = true;

        } else if id == 100 {
            //mul_assign_5(&mut account_struct[..], f1_r_range, cubic_range_0, cubic_range_1);
            mul_assign_5(
                &mut account_struct.f1_r_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[f1_r_range_iter] = true;

        } else if id == 101 {
            custom_f_inverse_4(
                &mut account_struct.cubic_range_0_s,
                &account_struct.f_f2_range_s
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 102 {
            custom_f_inverse_5(
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s,
                &mut account_struct.f_f2_range_s,
            );
            account_struct.changed_variables[f_f2_range_iter] = true;


        } else if id == 103 {
            //verify_result_and_withdraw(&account_struct.f1_r_range_s);

        } else if id == 121 {

        }
    }
*/
