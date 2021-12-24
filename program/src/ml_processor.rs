use crate::ml_instructions::*;
use crate::ml_instructions_transform::*;
use crate::ml_ranges::*;
use crate::ml_state::*;
use solana_program::{log::sol_log_compute_units, msg};
use crate::pi_state::PiBytes;
use crate::ml_parsers::parse_x_group_affine_from_bytes;
use crate::ml_parsers::parse_fp256_to_bytes;
use crate::ml_parsers::parse_f_from_bytes;
use crate::ml_parsers::parse_f_to_bytes;

pub fn move_proofs(account_main_data: &mut ML254Bytes, account_prepare_inputs_data: &PiBytes) {
    let proof_a = parse_x_group_affine_from_bytes(&account_prepare_inputs_data.proof_a_b_c_leaves_and_nullifiers[..64].to_vec());
    let proof_c = parse_x_group_affine_from_bytes(&account_prepare_inputs_data.proof_a_b_c_leaves_and_nullifiers[192..256].to_vec());
    let p_1: ark_ec::bn::G1Prepared<ark_bn254::Parameters> =
        ark_ec::bn::g1::G1Prepared::from(proof_a);
    let p_3: ark_ec::bn::G1Prepared<ark_bn254::Parameters> =
        ark_ec::bn::g1::G1Prepared::from(proof_c);

    parse_fp256_to_bytes(p_1.0.x, &mut account_main_data.p_1_x_range);
    parse_fp256_to_bytes(p_1.0.y, &mut account_main_data.p_1_y_range);
    parse_fp256_to_bytes(p_3.0.x, &mut account_main_data.p_3_x_range);
    parse_fp256_to_bytes(p_3.0.y, &mut account_main_data.p_3_y_range);
    // account_main_data.p_1_x_range = account_prepare_inputs_data.proof_a_b_c_leaves_and_nullifiers[..32].to_vec().clone();
    // account_main_data.p_1_y_range = account_prepare_inputs_data.proof_a_b_c_leaves_and_nullifiers[32..64].to_vec().clone();
    account_main_data.changed_variables[P_1_Y_RANGE_INDEX] = true;
    account_main_data.changed_variables[P_1_X_RANGE_INDEX] = true;

    account_main_data.proof_b = account_prepare_inputs_data.proof_a_b_c_leaves_and_nullifiers[64..192].to_vec().clone();
    account_main_data.changed_variables[PROOF_B_INDEX] = true;

    // account_main_data.p_3_x_range = account_prepare_inputs_data.proof_a_b_c_leaves_and_nullifiers[192..224].to_vec().clone();
    // account_main_data.p_3_y_range = account_prepare_inputs_data.proof_a_b_c_leaves_and_nullifiers[224..256].to_vec().clone();
    account_main_data.changed_variables[P_3_Y_RANGE_INDEX] = true;
    account_main_data.changed_variables[P_3_X_RANGE_INDEX] = true;

    let mut f_arr: Vec<u8> = vec![0; 384];
    f_arr[0] = 1;

    let f = parse_f_from_bytes(&mut f_arr);
    parse_f_to_bytes(f, &mut account_main_data.f_range);
    account_main_data.changed_variables[F_RANGE_INDEX] = true;
}

pub fn _process_instruction(
    id: u8,
    account_main: &mut ML254Bytes,
    // proof_b_bytes: &Vec<u8>,
    // p_1_bytes: &Vec<u8>,
    // p_3_bytes: &Vec<u8>,
) {
    msg!("in processor.");
    msg!("instruction: {:?}", id);
    sol_log_compute_units();
    if id == 0 {
        // First instruction of miller_loop.
        // Reads gic_affine from prepared_inputs account.
        // This Ix has been moved to the pre_processor.
    } else if id == 1 {
        // Inits proof_a and proof_c into the account.
        // (p1,p3)
        // Also inits f.
        // initialize_p1_p3_f_instruction(
        //     // &p_1_bytes,
        //     // &p_3_bytes,
        //     &mut account_main.p_1_x_range,
        //     &mut account_main.p_1_y_range,
        //     &mut account_main.p_3_x_range,
        //     &mut account_main.p_3_y_range,
        //     &mut account_main.f_range,
        // );
        // account_main.changed_variables[P_1_X_RANGE_INDEX] = true;
        // account_main.changed_variables[P_1_Y_RANGE_INDEX] = true;
        // account_main.changed_variables[P_3_X_RANGE_INDEX] = true;
        // account_main.changed_variables[P_3_Y_RANGE_INDEX] = true;
        // account_main.changed_variables[F_RANGE_INDEX] = true;
    } else if id == 2 {
        // Parses proof_b bytes into the account.(p1)
        // Called once at the beginning. Moved from 237
        init_coeffs1(
            &mut account_main.r,
            &mut account_main.proof_b,
            /*proof_b_bytes,*/
        );
        account_main.changed_variables[R_RANGE_INDEX] = true;
        account_main.changed_variables[PROOF_B_INDEX] = true;
    } else if id == 3 {
        square_in_place_instruction(&mut account_main.f_range);
        account_main.changed_variables[F_RANGE_INDEX] = true;
    } else if id == 4 {
        // The following three ix calls (4 and 5 and 6) each execute the same ELL loop.
        // Ix 4 executes the ELL loop for the first coeffs pair of 3.
        // Since the coeffs1/2/3 come from a proof.b compute this ix
        // call is pre-run by a computation ix:
        // either "doubling_step" (ix 7) or "addition_step" (ix 8 or 9 or 10 or 11)
        // The call_order of those is based on a constant as defined in the ark_ec library.
        ell_instruction_d(
            &mut account_main.f_range,
            &account_main.coeff_0_range,
            &account_main.coeff_1_range,
            &account_main.coeff_2_range,
            &account_main.p_1_y_range,
            &account_main.p_1_x_range,
        );
        account_main.changed_variables[F_RANGE_INDEX] = true;
    } else if id == 5 {
        // This ix (5) as well as ix 6 work a little differently. That's because here the ell loop derives
        // the coeff1/2/3 values not from a on-the-fly computation. It instead
        // reads the respective values from a hardcoded verifying key that's stored onchain.
        ell_instruction_d_c2(
            &mut account_main.f_range,
            &account_main.p_2_y_range,
            &account_main.p_2_x_range,
            &mut account_main.current_coeff_2_range,
        );
        account_main.changed_variables[F_RANGE_INDEX] = true;
        account_main.changed_variables[CURRENT_COEFF_2_RANGE_INDEX] = true;
    } else if id == 6 {
        // Works analogue to ix 5, but reads from a different part of the verifying key.
        ell_instruction_d_c3(
            &mut account_main.f_range,
            &account_main.p_3_y_range,
            &account_main.p_3_x_range,
            &mut account_main.current_coeff_3_range,
        );
        account_main.changed_variables[F_RANGE_INDEX] = true;
        account_main.changed_variables[CURRENT_COEFF_3_RANGE_INDEX] = true;
    } else if id == 7 {
        doubling_step(
            &mut account_main.r,
            &mut account_main.coeff_0_range,
            &mut account_main.coeff_1_range,
            &mut account_main.coeff_2_range,
        );
        account_main.changed_variables[R_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_0_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_1_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_2_RANGE_INDEX] = true;
    } else if id == 8 {
        // The reason "addition_step" needs 4 different ix calls (8/9/10/11)
        // is that we need to parse in a unique flag based on which precompute
        // needs to be done with the &q value. This is analogue to the
        // ark_ec library implementation for bn254.
        addition_step::<ark_bn254::Parameters>(
            &mut account_main.coeff_0_range,
            &mut account_main.coeff_1_range,
            &mut account_main.coeff_2_range,
            &mut account_main.r,
            &account_main.proof_b,
            "normal",
        );
        account_main.changed_variables[R_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_0_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_1_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_2_RANGE_INDEX] = true;
    } else if id == 9 {
        addition_step::<ark_bn254::Parameters>(
            &mut account_main.coeff_0_range,
            &mut account_main.coeff_1_range,
            &mut account_main.coeff_2_range,
            &mut account_main.r,
            &account_main.proof_b,
            "negq",
        );
        account_main.changed_variables[R_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_0_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_1_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_2_RANGE_INDEX] = true;
    } else if id == 10 {
        addition_step::<ark_bn254::Parameters>(
            &mut account_main.coeff_0_range,
            &mut account_main.coeff_1_range,
            &mut account_main.coeff_2_range,
            &mut account_main.r,
            &account_main.proof_b,
            "q1",
        );
        account_main.changed_variables[R_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_0_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_1_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_2_RANGE_INDEX] = true;
    } else if id == 11 {
        addition_step::<ark_bn254::Parameters>(
            &mut account_main.coeff_0_range,
            &mut account_main.coeff_1_range,
            &mut account_main.coeff_2_range,
            &mut account_main.r,
            &account_main.proof_b,
            "q2",
        );
        account_main.changed_variables[R_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_0_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_1_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_2_RANGE_INDEX] = true;
    }
    msg!("compute costs:");
    sol_log_compute_units();
}

// pub fn _process_instruction_bridge_to_final_exp(
//     id: u8,
//     account_main_data: &mut MillerLoopBytes,
//     account_verify_part_2_data: &mut FinalExpBytes,
// ) {
//     if id == 255 {
//         // pass final f to account_verif

//         let data = &account_main_data.f_range;
//         // assert_eq!(data,&vec![]);

//         for i in 0..576 {
//             account_verify_part_2_data.f1_r_range_s[i] = data[i].clone();
//             account_verify_part_2_data.f_f2_range_s[i] = data[i].clone();
//             account_verify_part_2_data.i_range_s[i] = 0u8;
//             account_verify_part_2_data.y0_range_s[i] = 0u8;
//             account_verify_part_2_data.y1_range_s[i] = 0u8;
//             account_verify_part_2_data.y2_range_s[i] = 0u8;
//             // assert_eq!(data[i], account_verify_part_2_data.f1_r_range_s[i]);
//         }

//         account_verify_part_2_data.cubic_range_0_s = vec![0u8; 288];
//         account_verify_part_2_data.cubic_range_1_s = vec![0u8; 288];
//         account_verify_part_2_data.cubic_range_2_s = vec![0u8; 288];

//         account_verify_part_2_data.quad_range_0_s = vec![0u8; 96];
//         account_verify_part_2_data.quad_range_1_s = vec![0u8; 96];
//         account_verify_part_2_data.quad_range_2_s = vec![0u8; 96];
//         account_verify_part_2_data.quad_range_3_s = vec![0u8; 96];

//         account_verify_part_2_data.fp384_range_s = vec![0u8; 48];

//         for i in 0..14 {
//             account_verify_part_2_data.changed_variables[i] = true;
//         }

//         // msg!("0 {}", account_verify_part_2_data.f1_r_range_s[0] );
//         // msg!("566 {}", account_verify_part_2_data.f1_r_range_s[566] );
//     }
// }
