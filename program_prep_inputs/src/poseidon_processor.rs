use crate::poseidon_instructions::*;
use crate::state_prep_inputs::{PoseidonHashBytesPrepInputs};
use crate::poseidon_parsers::*;
use solana_program::{
    msg,
    log::sol_log_compute_units,
};

pub fn processor(id: u8, account: &mut PoseidonHashBytesPrepInputs){
    if id == 0 {
       init_sponge(account);
   }
    else if id == 1 {
        absorb_instruction_vec_22_0(&mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3,  &[account.amount, account.relayer_refund].concat() /*&vec![0u8;16]*/);
        // account.changed_variables[cubic_range_1_iter] = true;
        // println!("1");
    }else if id == 2 {
        //absorb_instruction_vec_22_1(&mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3, &account.to_address, &account.signing_address);
        absorb_internal_custom_0(&mut account.state_range_1, &mut account.state_range_2, &mut account.state_range_3, &account.to_address.to_vec(), &account.signing_address.to_vec(), &mut account.fp256_0, &mut account.fp256_1);

    } else if id == 24 {
        absorb_internal_custom_1(&mut account.state_range_1, &mut account.state_range_2, &mut account.state_range_3, &mut account.fp256_0, &mut account.fp256_1);

    } else if id == 3 {
        permute_instruction_1_and_3(0, &mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3);
        // account.changed_variables[cubic_range_1_iter] = true;
        // println!("3");
    } else if id == 4 {
        permute_instruction_1_and_3(2, &mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3);
        // account.changed_variables[cubic_range_1_iter] = true;
        // println!("4");
    } else if id == 5 {
        permute_instruction_2_x_4(4, &mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3);
        // account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 6 {
        permute_instruction_2_x_4(8, &mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3);
        // account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 7 {
        permute_instruction_2_x_4(12, &mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3);
        // account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 8 {
        permute_instruction_2_x_4(16, &mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3);
        // account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 9 {
        permute_instruction_2_x_4(20, &mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3);
        // account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 10 {
        permute_instruction_2_x_4(24, &mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3);
        // account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 11 {
        permute_instruction_2_x_4(28, &mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3);
        // account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 12 {
        permute_instruction_2_x_4(32, &mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3);
        // account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 13 {
        permute_instruction_2_x_4(36, &mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3);
        // account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 14 {
        permute_instruction_2_x_4(40, &mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3);
        // account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 15 {
        permute_instruction_2_x_4(44, &mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3);
        // account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 16 {
        permute_instruction_2_x_4(48, &mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3);
        // account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 17 {
        permute_instruction_2_x_4(52, &mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3);
        // account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 18 {
        permute_instruction_2_x_4(56, &mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3);
        // account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 30 {
        permute_instruction_2_x_4(60, &mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3);
        // account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 31 {
        permute_instruction_2_x_4(64, &mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3);
        // account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 32 {
        permute_instruction_2_x_4(68, &mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3);
        // account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 33 {
        permute_instruction_2_x_2(72, &mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3);
        // account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 19 { // *3
        permute_instruction_2_x_3(74, &mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3);
        // account.changed_variables[cubic_range_1_iter] = true;
    } else if id == 20 {
        permute_instruction_1_and_3(77, &mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3);
        // account.changed_variables[cubic_range_1_iter] = true;
        // println!("20");
    } else if id == 21 {
        permute_instruction_1_and_3(79, &mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3);
        // account.changed_variables[cubic_range_1_iter] = true;
        // println!("21");
    }  else if id == 23 {
        let reference = account.tx_integrity_hash.clone();
        squeeze_internal_custom(&mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3, &mut account.tx_integrity_hash, 0);
        assert_eq!(reference, account.tx_integrity_hash);
        // account.changed_variables[cubic_range_1_iter] = true;
        // println!("21");
    }/* else if id == 24 {
        absorb_instruction_squeeze_field_elem_24(&mut account.state_range_1 , &mut account.state_range_2, &mut account.state_range_3, &mut account.result, 0);
        // account.changed_variables[cubic_range_1_iter] = true;
        // println!("21");
    }*/

}
