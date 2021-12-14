use crate::ml_254_instructions::*;
use crate::ml_254_parsers::*;
use crate::ml_254_processor::*;
use crate::ml_254_ranges::*;
use crate::ml_254_state::*;
use crate::pi_254_state_COPY::*;
// use crate::state_miller_loop_transfer::MillerLoopTransferBytes;
// use crate::state_prep_inputs;
// use crate::state_prep_inputs::PrepareInputsBytes;
// use crate::FinalExpBytes; //state_final_exp::FinalExpBytes;
use solana_program::program_pack::Pack;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    log::sol_log_compute_units,
    msg,
    program_error::ProgramError,
};

pub fn _pre_process_instruction_miller_loop(
    _instruction_data: &[u8],
    accounts: &[AccountInfo],
) -> Result<(), ProgramError> {
    let account = &mut accounts.iter();
    let signing_account = next_account_info(account)?;
    let account_main = next_account_info(account)?;
    msg!(
        "new ix -- IX_DATA ARRIVED: {:?}",
        _instruction_data[..].to_vec()
    );
    msg!("unpacking");
    let mut account_main_data = ML254Bytes::unpack(&account_main.data.borrow())?;
    msg!("unpacked");

    // assert!(
    //     account_main_data.current_instruction_index < 1821,
    //     "Miller loop finished"
    // );
    // let mut ix_order_array = [];

    // First ix: "0" -> Parses g_ic_affine from prepared_inputs.
    // Hardcoded for test purposes.
    if ML_IX_ORDER[account_main_data.current_instruction_index] == 0 {
        // let mut account_main_data = MillerLoopTransferBytes::unpack(&account_main.data.borrow())?; // ..7081
        let account_prepare_inputs = next_account_info(account)?;
        let account_prepare_inputs_data = PiBytes::unpack(&account_prepare_inputs.data.borrow())?;

        // //signer which completed prepared inputs is the same as of this tx
        // assert_eq!(
        //     *account_prepare_inputs.owner,
        //     solana_program::pubkey::Pubkey::new(&state_prep_inputs::PREPARED_INPUTS_PUBKEY[..])
        // );
        // //signer is equal
        // assert_eq!(
        //     *signing_account.key,
        //     solana_program::pubkey::Pubkey::new(&account_prepare_inputs_data.signing_address),
        //     "Invalid sender"
        // );
        // //root hash been found in prepared inputs
        // assert_eq!(
        //     account_prepare_inputs_data.found_root, 1,
        //     "No root was found in prior algorithm"
        // );
        // //prepared inputs has been completed
        // assert_eq!(
        //     account_prepare_inputs_data.current_instruction_index, 1809,
        //     "prepare inputs is not completed yet"
        // );

        let g_ic_affine = parse_x_group_affine_from_bytes(&account_prepare_inputs_data.x_1_range); // 10k
        let p2: ark_ec::bn::G1Prepared<ark_bn254::Parameters> =
            ark_ec::bn::g1::G1Prepared::from(g_ic_affine);
        msg!(
            "prepared inputs bytes:{:?}",
            account_prepare_inputs_data.x_1_range
        );
        // //assert_eq!(true, false);
        // account_main_data.found_root = account_prepare_inputs_data.found_root.clone();
        // account_main_data.found_nullifier = account_prepare_inputs_data.found_nullifier.clone();
        // account_main_data.executed_withdraw = account_prepare_inputs_data.executed_withdraw.clone();
        // account_main_data.signing_address = account_prepare_inputs_data.signing_address.clone();
        // account_main_data.relayer_refund = account_prepare_inputs_data.relayer_refund.clone();
        // account_main_data.to_address = account_prepare_inputs_data.to_address.clone();
        // account_main_data.amount = account_prepare_inputs_data.amount.clone();
        // account_main_data.nullifier_hash = account_prepare_inputs_data.nullifier_hash.clone();
        // account_main_data.root_hash = account_prepare_inputs_data.root_hash.clone();
        // account_main_data.data_hash = account_prepare_inputs_data.data_hash.clone();
        // account_main_data.tx_integrity_hash = account_prepare_inputs_data.tx_integrity_hash.clone();

        parse_fp256_to_bytes(p2.0.x, &mut account_main_data.p_2_x_range);
        parse_fp256_to_bytes(p2.0.y, &mut account_main_data.p_2_y_range);
        account_main_data.current_instruction_index += 1;
        // MillerLoopTransferBytes::pack_into_slice(
        //     &account_main_data,
        //     &mut account_main.data.borrow_mut(),
        // );
        account_main_data.changed_variables[P_2_Y_RANGE_INDEX] = true;
        account_main_data.changed_variables[P_2_X_RANGE_INDEX] = true;
        // account_main_data.changed_variables[P_2_Y_RANGE] = true;

        ML254Bytes::pack_into_slice(&account_main_data, &mut account_main.data.borrow_mut());
        return Ok(());
    }
    // Passes final f to account_verif2. Skipped for testing purposes.
    // else if ML_IX_ORDER[account_main_data.current_instruction_index] == 255 {
    //     msg!("last ix of ml (255), reading f...");
    //     let f = parse_f_from_bytes(account_main_data.f_range);
    //     // assert_eq!(f,Null);
    //     //let account_verify_part_2 = next_account_info(account)?;
    //     //will just transfer bytes to the right place and zero out the rest
    //     // let mut account_verify_part_2_data = FinalExpBytes::unpack(&account_main.data.borrow())?; // 0

    //     // _process_instruction_bridge_to_final_exp(
    //     //     complete_instruction_order_verify_one[account_main_data.current_instruction_index],
    //     //     &mut account_main_data,
    //     //     &mut account_verify_part_2_data,
    //     // );
    //     // FinalExpBytes::pack_into_slice(
    //     //     &account_verify_part_2_data,
    //     //     &mut account_main.data.borrow_mut(),
    //     // );
    // }
    else {
        // Empty vecs that pass data from the client if called with respective ix.
        let mut proof_b_bytes = vec![];
        let mut p_1_bytes = vec![];
        let mut p_3_bytes = vec![];

        if ML_IX_ORDER[account_main_data.current_instruction_index] == 1 {
            p_1_bytes = _instruction_data[10..74].to_vec(); // 2..194 (192 ) // are 128 => 2..130 BUT starting at 10 bc
            p_3_bytes = _instruction_data[74..138].to_vec();
        }
        if ML_IX_ORDER[account_main_data.current_instruction_index] == 2 {
            proof_b_bytes = _instruction_data[10..138].to_vec(); // 2..194 => 2..130 (bc proofb now 128) => 10..138
        }

        if ML_IX_ORDER[account_main_data.current_instruction_index] == 3 {
            // assert that p1,3,proof.b and p2(prepared inputs) are eq
            // account_main_data,
        }
        _process_instruction(
            ML_IX_ORDER[account_main_data.current_instruction_index],
            &mut account_main_data,
            &proof_b_bytes,
            &p_1_bytes,
            &p_3_bytes,
        ); // updated: will always pass coeff quads as empty. Acc gets them from prior instructions.    }
           // msg!(
           //     "Instruction {}",
           //     ML_IX_ORDER[account_main_data.current_instruction_index]
           // );

        //checks signer
        // assert_eq!(
        //     *signing_account.key,
        //     solana_program::pubkey::Pubkey::new(&account_main_data.signing_address),
        //     "Invalid sender"
        // );

        account_main_data.current_instruction_index += 1;
        //resetting instruction index to be able to reuse account
        if account_main_data.current_instruction_index == 430 {
            account_main_data.current_instruction_index = 0;

        }
        msg!("packing");
        ML254Bytes::pack_into_slice(&account_main_data, &mut account_main.data.borrow_mut());
        msg!("packed");
        Ok(())
    }
}

// const ML_IX_ORDER: [u8; 430] = [
//     0, 1, 2, 73, 69, 70, 71, 74, 69, 70, 71, 72, 73, 69, 70, 71, 72, 73, 69, 70, 71, 74, 69, 70,
//     71, 72, 73, 69, 70, 71, 72, 73, 69, 70, 71, 72, 73, 69, 70, 71, 75, 69, 70, 71, 72, 73, 69, 70,
//     71, 72, 73, 69, 70, 71, 74, 69, 70, 71, 72, 73, 69, 70, 71, 74, 69, 70, 71, 72, 73, 69, 70, 71,
//     72, 73, 69, 70, 71, 72, 73, 69, 70, 71, 72, 73, 69, 70, 71, 75, 69, 70, 71, 72, 73, 69, 70, 71,
//     72, 73, 69, 70, 71, 72, 73, 69, 70, 71, 74, 69, 70, 71, 72, 73, 69, 70, 71, 74, 69, 70, 71, 72,
//     73, 69, 70, 71, 72, 73, 69, 70, 71, 72, 73, 69, 70, 71, 75, 69, 70, 71, 72, 73, 69, 70, 71, 72,
//     73, 69, 70, 71, 72, 73, 69, 70, 71, 72, 73, 69, 70, 71, 72, 73, 69, 70, 71, 72, 73, 69, 70, 71,
//     74, 69, 70, 71, 72, 73, 69, 70, 71, 72, 73, 69, 70, 71, 72, 73, 69, 70, 71, 75, 69, 70, 71, 72,
//     73, 69, 70, 71, 72, 73, 69, 70, 71, 72, 73, 69, 70, 71, 74, 69, 70, 71, 72, 73, 69, 70, 71, 74,
//     69, 70, 71, 72, 73, 69, 70, 71, 74, 69, 70, 71, 72, 73, 69, 70, 71, 72, 73, 69, 70, 71, 72, 73,
//     69, 70, 71, 72, 73, 69, 70, 71, 72, 73, 69, 70, 71, 75, 69, 70, 71, 72, 73, 69, 70, 71, 72, 73,
//     69, 70, 71, 74, 69, 70, 71, 72, 73, 69, 70, 71, 72, 73, 69, 70, 71, 72, 73, 69, 70, 71, 75, 69,
//     70, 71, 72, 73, 69, 70, 71, 72, 73, 69, 70, 71, 74, 69, 70, 71, 72, 73, 69, 70, 71, 74, 69, 70,
//     71, 72, 73, 69, 70, 71, 72, 73, 69, 70, 71, 72, 73, 69, 70, 71, 74, 69, 70, 71, 72, 73, 69, 70,
//     71, 72, 73, 69, 70, 71, 72, 73, 69, 70, 71, 75, 69, 70, 71, 72, 73, 69, 70, 71, 74, 69, 70, 71,
//     72, 73, 69, 70, 71, 72, 73, 69, 70, 71, 72, 73, 69, 70, 71, 75, 69, 70, 71, 72, 73, 69, 70, 71,
//     72, 73, 69, 70, 71, 74, 69, 70, 71, 72, 73, 69, 70, 71, 72, 73, 69, 70, 71, 74, 69, 70, 71, 72,
//     73, 69, 70, 71, 72, 73, 69, 70, 71, 72, 73, 69, 70, 71, 76, 69, 70, 71, 77, 69, 70, 71,
// ];

pub const ML_IX_ORDER: [u8; 430] = [
    0, 1, 2, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7,
    4, 5, 6, 3, 7, 4, 5, 6, 9, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 8,
    4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 9, 4, 5, 6, 3, 7, 4, 5, 6,
    3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5,
    6, 3, 7, 4, 5, 6, 9, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7,
    4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 9, 4, 5, 6, 3,
    7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6,
    8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 9, 4, 5,
    6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 9, 4,
    5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7,
    4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 9, 4, 5, 6, 3,
    7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 9, 4, 5, 6, 3, 7, 4, 5, 6,
    3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5,
    6, 3, 7, 4, 5, 6, 10, 4, 5, 6, 11, 4, 5, 6,
];
