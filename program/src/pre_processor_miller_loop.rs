use crate::instructions::*;
use crate::parsers::*;
use crate::state_miller_loop::{complete_instruction_order_verify_one, MillerLoopBytes};
use crate::state_miller_loop_transfer::MillerLoopTransferBytes;
use crate::state_prep_inputs::PrepareInputsBytes;
//use crate::state_merkle_tree;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    log::sol_log_compute_units,
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};

use crate::parsers_prepare_inputs::parse_x_group_affine_from_bytes;
use crate::processor::{_process_instruction, _process_instruction_bridge_to_final_exp};
use crate::ranges::*;
use crate::state_prep_inputs;
use crate::FinalExpBytes; //state_final_exp::FinalExpBytes;

pub fn _pre_process_instruction_miller_loop(
    _instruction_data: &[u8],
    accounts: &[AccountInfo],
) -> Result<(), ProgramError> {
    let account = &mut accounts.iter();
    let signing_account = next_account_info(account)?;
    let account_main = next_account_info(account)?;
    let mut account_main_data = MillerLoopBytes::unpack(&account_main.data.borrow())?;
    assert!(
        account_main_data.current_instruction_index < 1821,
        "Miller loop finished"
    );

    let account = &mut accounts.iter();
    let account1 = next_account_info(account)?;
    let account_main = next_account_info(account)?; // always the storage account no matter which part (1,2, merkletree)

    // parse g_ic_affine from prepared_inputs
    if complete_instruction_order_verify_one[account_main_data.current_instruction_index] == 251 {
        let mut account_main_data = MillerLoopTransferBytes::unpack(&account_main.data.borrow())?; // ..7081
        let account_prepare_inputs = next_account_info(account)?;
        let mut account_prepare_inputs_data =
            PrepareInputsBytes::unpack(&account_prepare_inputs.data.borrow())?;

        //signer which completed prepared inputs is the same as of this tx
        assert_eq!(
            *account_prepare_inputs.owner,
            solana_program::pubkey::Pubkey::new(&state_prep_inputs::PREPARED_INPUTS_PUBKEY[..])
        );
        //signer is equal
        assert_eq!(
            *signing_account.key,
            solana_program::pubkey::Pubkey::new(&account_prepare_inputs_data.signing_address),
            "Invalid sender"
        );
        //root hash been found in prepared inputs
        assert_eq!(
            account_prepare_inputs_data.found_root, 1,
            "No root was found in prior algorithm"
        );
        //prepared inputs has been completed
        assert_eq!(
            account_prepare_inputs_data.current_instruction_index, 1809,
            "prepare inputs is not completed yet"
        );

        let g_ic_affine = parse_x_group_affine_from_bytes(&account_prepare_inputs_data.x_1_range); // 10k
        let p2: ark_ec::bls12::G1Prepared<ark_bls12_381::Parameters> =
            ark_ec::bls12::g1::G1Prepared::from(g_ic_affine);
        msg!(
            "prepared inputs bytes:{:?}",
            account_prepare_inputs_data.x_1_range
        );
        //assert_eq!(true, false);
        parse_fp384_to_bytes(p2.0.x, &mut account_main_data.p_2_x_range);
        parse_fp384_to_bytes(p2.0.y, &mut account_main_data.p_2_y_range);
        account_main_data.found_root = account_prepare_inputs_data.found_root.clone();
        account_main_data.found_nullifier = account_prepare_inputs_data.found_nullifier.clone();
        account_main_data.executed_withdraw = account_prepare_inputs_data.executed_withdraw.clone();
        account_main_data.signing_address = account_prepare_inputs_data.signing_address.clone();
        account_main_data.relayer_refund = account_prepare_inputs_data.relayer_refund.clone();
        account_main_data.to_address = account_prepare_inputs_data.to_address.clone();
        account_main_data.amount = account_prepare_inputs_data.amount.clone();
        account_main_data.nullifier_hash = account_prepare_inputs_data.nullifier_hash.clone();
        account_main_data.root_hash = account_prepare_inputs_data.root_hash.clone();
        account_main_data.data_hash = account_prepare_inputs_data.data_hash.clone();
        account_main_data.tx_integrity_hash = account_prepare_inputs_data.tx_integrity_hash.clone();

        account_main_data.current_instruction_index += 1;
        MillerLoopTransferBytes::pack_into_slice(
            &account_main_data,
            &mut account_main.data.borrow_mut(),
        );
        return Ok(());
    }
    // // init f
    else if complete_instruction_order_verify_one[account_main_data.current_instruction_index]
        == 0
    {
        let mut f_arr: Vec<u8> = vec![0; 576];
        f_arr[0] = 1;
        account_main_data.f_range = f_arr;
        account_main_data.changed_variables[F_RANGE_INDEX] = true;
    }
    // // init p1,p2:
    else if complete_instruction_order_verify_one[account_main_data.current_instruction_index]
        == 230
    {
        // unpack main
        //let mut account_main_data = MillerLoopBytes::unpack(&account_main.data.borrow())?;
        // turn bytes into affine, then into prepared, parse prepared

        let proof_a = parse_x_group_affine_from_bytes(&_instruction_data[2..98].to_vec());
        let proof_c = parse_x_group_affine_from_bytes(&_instruction_data[98..194].to_vec());

        let p1: ark_ec::bls12::G1Prepared<ark_bls12_381::Parameters> =
            ark_ec::bls12::g1::G1Prepared::from(proof_a);
        let p3: ark_ec::bls12::G1Prepared<ark_bls12_381::Parameters> =
            ark_ec::bls12::g1::G1Prepared::from(proof_c);

        parse_fp384_to_bytes(p1.0.x, &mut account_main_data.p_1_x_range);
        parse_fp384_to_bytes(p1.0.y, &mut account_main_data.p_1_y_range);
        parse_fp384_to_bytes(p3.0.x, &mut account_main_data.p_3_x_range);
        parse_fp384_to_bytes(p3.0.y, &mut account_main_data.p_3_y_range);

        account_main_data.changed_variables[P_1_X_RANGE_INDEX] = true;
        account_main_data.changed_variables[P_1_Y_RANGE_INDEX] = true;
        account_main_data.changed_variables[P_3_X_RANGE_INDEX] = true;
        account_main_data.changed_variables[P_3_Y_RANGE_INDEX] = true;

        // init f
        let mut f_arr: Vec<u8> = vec![0; 576];
        f_arr[0] = 1;
        account_main_data.f_range = f_arr;
        account_main_data.changed_variables[F_RANGE_INDEX] = true;

        // pack main
        //MillerLoopBytes::pack_into_slice(&account_main_data, &mut account_main.data.borrow_mut());
    }
    // pass final f to account_verif2
    else if complete_instruction_order_verify_one[account_main_data.current_instruction_index]
        == 255
    {
        // account_main is actually a separate account

        //let account_verify_part_2 = next_account_info(account)?;
        //will just transfer bytes to the right place and zero out the rest
        let mut account_verify_part_2_data = FinalExpBytes::unpack(&account_main.data.borrow())?; // 0

        _process_instruction_bridge_to_final_exp(
            complete_instruction_order_verify_one[account_main_data.current_instruction_index],
            &mut account_main_data,
            &mut account_verify_part_2_data,
        );
        FinalExpBytes::pack_into_slice(
            &account_verify_part_2_data,
            &mut account_main.data.borrow_mut(),
        );
    }
    // NEW INSTRUCTION:
    else {
        // dead code:
        let mut current_coeff_quad_0 = vec![];
        let mut current_coeff_quad_1 = vec![];
        let mut current_coeff_quad_2 = vec![];

        let mut proof_b_bytes = vec![];

        if complete_instruction_order_verify_one[account_main_data.current_instruction_index] == 237
        {
            // let proof_b = parse_proof_b_from_bytes(&_instruction_data[2..194].to_vec());
            proof_b_bytes = _instruction_data[2..194].to_vec();
        }
        // msg!("v1: {:?}", &account_main_data.f_range);
        _process_instruction(
            complete_instruction_order_verify_one[account_main_data.current_instruction_index],
            &mut account_main_data,
            &current_coeff_quad_0,
            &current_coeff_quad_1,
            &current_coeff_quad_2,
            &proof_b_bytes,
        ); // updated: will always pass coeff quads as empty. Acc gets them from prior instructions.

        if complete_instruction_order_verify_one[account_main_data.current_instruction_index] == 233
        {
            /*
            assert_eq!(get_hardcoded_coeffs::get_hardcoded_coeffs()[0..96], current_coeff_quad_0[..]);
            assert_eq!(get_hardcoded_coeffs::get_hardcoded_coeffs()[96..182], current_coeff_quad_1[..]);
            assert_eq!(get_hardcoded_coeffs::get_hardcoded_coeffs()[182..288], current_coeff_quad_2[..]);
            */
        }
    }
    msg!(
        "Instruction {}",
        complete_instruction_order_verify_one[account_main_data.current_instruction_index]
    );

    //checks signer
    assert_eq!(
        *signing_account.key,
        solana_program::pubkey::Pubkey::new(&account_main_data.signing_address),
        "Invalid sender"
    );

    account_main_data.current_instruction_index += 1;
    MillerLoopBytes::pack_into_slice(&account_main_data, &mut account_main.data.borrow_mut());
    //msg!("Instruction {}", account_main_data.current_instruction_index);
    Ok(())
}
