use crate::ml_254_instructions::*;
use crate::ml_254_instructions_transform::*;
use crate::ml_254_ranges::*;
use crate::ml_254_state::*;
use solana_program::{log::sol_log_compute_units, msg};

pub fn _process_instruction(
    id: u8,
    account_main: &mut ML254Bytes,
    proof_b_bytes: &Vec<u8>,
    p_1_bytes: &Vec<u8>,
    p_3_bytes: &Vec<u8>,
) {
    msg!("in processor.");
    msg!("instruction: {:?}", id);
    sol_log_compute_units();
    if id == 0 {
        // First instruction for the millerloop.
        // Reads gic_affine from prepared_inputs account.
        // Ix moved to pre-processor.
    } else if id == 1 {
        // Inits proof_a and proof_c into the account.
        // (p1,p3)
        // Also inits f.
        initialize_p1_p3_f_instruction(
            &p_1_bytes,
            &p_3_bytes,
            &mut account_main.p_1_x_range,
            &mut account_main.p_1_y_range,
            &mut account_main.p_3_x_range,
            &mut account_main.p_3_y_range,
            &mut account_main.f_range,
        );

        account_main.changed_variables[P_1_X_RANGE_INDEX] = true;
        account_main.changed_variables[P_1_Y_RANGE_INDEX] = true;
        account_main.changed_variables[P_3_X_RANGE_INDEX] = true;
        account_main.changed_variables[P_3_Y_RANGE_INDEX] = true;
        account_main.changed_variables[F_RANGE_INDEX] = true;
    } else if id == 2 {
        // Parses proof_b bytes into the account.(p1)
        // Called once at the beginning. Moved from 237
        init_coeffs1(
            &mut account_main.r,
            &mut account_main.proof_b,
            &mut account_main.proof_b_tmp_range,
            proof_b_bytes,
        );
        // account_main.changed_variables[20] = true;
        // account_main.changed_variables[21] = true;
        // account_main.changed_variables[22] = true;
        account_main.changed_variables[R_RANGE_INDEX] = true;
        account_main.changed_variables[PROOF_B_INDEX] = true;
        account_main.changed_variables[PROOF_B_TMP_RANGE_INDEX] = true;

        // Next: 3,4 are ix that replicate .square_in_place()
    } else if id == 3 {
        custom_square_in_place_instruction_else_1(
            &account_main.f_range,
            &mut account_main.cubic_v0_range,
            &mut account_main.cubic_v2_range,
            &mut account_main.cubic_v3_range,
        );
        account_main.changed_variables[CUBIC_V0_RANGE_INDEX] = true;
        account_main.changed_variables[CUBIC_V2_RANGE_INDEX] = true;
        account_main.changed_variables[CUBIC_V3_RANGE_INDEX] = true;
    } else if id == 4 {
        custom_square_in_place_instruction_else_2(
            &account_main.cubic_v0_range,
            &account_main.cubic_v2_range,
            &account_main.cubic_v3_range,
            &mut account_main.f_range,
        );
        account_main.changed_variables[F_RANGE_INDEX] = true;
    } else if id == 5 {
    } else if id == 6 {
    } else if id == 7 {
        // Note that v0,v2,v3 ranges are overwritten by
        // the following instructions.
        custom_ell_instruction_D_2(
            &mut account_main.f_range,
            &mut account_main.coeff_0_range,
            &mut account_main.cubic_v0_range, // used as a_range
        );
        account_main.changed_variables[F_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_0_RANGE_INDEX] = true;
        account_main.changed_variables[CUBIC_V0_RANGE_INDEX] = true;
    } else if id == 8 {
        custom_ell_instruction_D_3(
            &mut account_main.cubic_v2_range, // used as b_range
            &account_main.f_range,
            &account_main.coeff_1_range,
            &account_main.coeff_2_range,
        );
        account_main.changed_variables[CUBIC_V2_RANGE_INDEX] = true;
    } else if id == 9 {
        custom_ell_instruction_D_4(
            &mut account_main.cubic_v3_range, // used as e_range
            &account_main.f_range,
            &account_main.coeff_0_range,
            &account_main.coeff_1_range,
            &account_main.coeff_2_range,
        );
        account_main.changed_variables[CUBIC_V3_RANGE_INDEX] = true;
    } else if id == 10 {
        custom_ell_instruction_D_5(
            &mut account_main.f_range,
            &account_main.cubic_v0_range, // used as a_range
            &account_main.cubic_v2_range, // used as b_range
            &account_main.cubic_v3_range, // used as e_range
        );
        account_main.changed_variables[F_RANGE_INDEX] = true;
    // (11)-(16) compute the current coeff_0.
    // (11)-(13) and (14)-(16) alternate.
    // So at any given round it's just calling 3 ix.
    } else if id == 11 {
        doubling_step_custom_0(
            &mut account_main.r,
            &mut account_main.h,
            &mut account_main.g,
            &mut account_main.e,
            &mut account_main.lambda,
            &mut account_main.theta,
        );
        account_main.changed_variables[R_RANGE_INDEX] = true;
        account_main.changed_variables[H_RANGE_INDEX] = true;
        account_main.changed_variables[G_RANGE_INDEX] = true;
        account_main.changed_variables[E_RANGE_INDEX] = true;
        account_main.changed_variables[LAMBDA_RANGE_INDEX] = true;
        account_main.changed_variables[THETA_RANGE_INDEX] = true;
    } else if id == 12 {
        doubling_step_custom_1(
            &mut account_main.r,
            &mut account_main.h,
            &mut account_main.g,
            &account_main.e,
            &account_main.lambda,
        );
        //5 6 8
        account_main.changed_variables[R_RANGE_INDEX] = true;
        account_main.changed_variables[H_RANGE_INDEX] = true;
        account_main.changed_variables[G_RANGE_INDEX] = true;
    } else if id == 13 {
        doubling_step_custom_2(
            &mut account_main.coeff_0_range,
            &mut account_main.coeff_1_range,
            &mut account_main.coeff_2_range,
            &account_main.h,
            &account_main.g,
            &account_main.theta,
        );
        account_main.changed_variables[COEFF_0_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_1_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_2_RANGE_INDEX] = true;
    } else if id == 14 {
        addition_step_custom_0::<ark_bn254::Parameters>(
            &account_main.r,
            &mut account_main.h,
            &mut account_main.g,
            &mut account_main.e,
            &mut account_main.lambda,
            &mut account_main.theta,
            &account_main.proof_b,
        );

        account_main.changed_variables[H_RANGE_INDEX] = true;
        account_main.changed_variables[G_RANGE_INDEX] = true;
        account_main.changed_variables[E_RANGE_INDEX] = true;
        account_main.changed_variables[LAMBDA_RANGE_INDEX] = true;
        account_main.changed_variables[THETA_RANGE_INDEX] = true;
        account_main.changed_variables[PROOF_B_INDEX] = true;
    } else if id == 15 {
        addition_step_custom_1::<ark_bn254::Parameters>(
            &mut account_main.r,
            &account_main.h,
            &account_main.g,
            &account_main.e,
            &account_main.lambda,
            &account_main.theta,
        );
        account_main.changed_variables[R_RANGE_INDEX] = true;
    } else if id == 16 {
        addition_step_custom_2::<ark_bn254::Parameters>(
            &mut account_main.coeff_0_range,
            &mut account_main.coeff_1_range,
            &mut account_main.coeff_2_range,
            &account_main.lambda,
            &account_main.theta,
            &account_main.proof_b,
        );
        account_main.changed_variables[COEFF_0_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_1_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_2_RANGE_INDEX] = true;

    // (17) and (18) compute coeffs_2 and coeffs_3 respectively.
    // This consumes less resources than (16) since they're
    // just reading the values from a hardcoded pvk.
    } else if id == 17 {
        instruction_onchain_coeffs_2(
            &mut account_main.current_coeff_2_range,
            &mut account_main.coeff_2_range,
            &mut account_main.coeff_1_range,
            &mut account_main.coeff_0_range,
        );

        account_main.changed_variables[COEFF_2_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_1_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_0_RANGE_INDEX] = true;
        account_main.changed_variables[CURRENT_COEFF_2_RANGE_INDEX] = true;
    } else if id == 18 {
        instruction_onchain_coeffs_3(
            &mut account_main.current_coeff_3_range,
            &mut account_main.coeff_2_range,
            &mut account_main.coeff_1_range,
            &mut account_main.coeff_0_range,
        );

        account_main.changed_variables[COEFF_2_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_1_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_0_RANGE_INDEX] = true;
        account_main.changed_variables[CURRENT_COEFF_3_RANGE_INDEX] = true;
    } else if id == 19 {
    }
    // Below ix (20,21,22) are called 91 times each
    // in alternating order.
    // They each draw the currently computed coeffs
    else if id == 20 {
        // For p_1
        custom_ell_instruction_D_1(
            &mut account_main.coeff_2_range,
            &mut account_main.coeff_1_range,
            &mut account_main.coeff_0_range,
            &account_main.p_1_y_range,
            &account_main.p_1_x_range,
        );

        account_main.changed_variables[COEFF_2_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_1_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_0_RANGE_INDEX] = true;
    } else if id == 21 {
        custom_ell_instruction_D_1(
            // For p_2
            &mut account_main.coeff_2_range,
            &mut account_main.coeff_1_range,
            &mut account_main.coeff_0_range,
            &account_main.p_2_y_range,
            &account_main.p_2_x_range,
        );

        account_main.changed_variables[COEFF_2_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_1_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_0_RANGE_INDEX] = true;
    } else if id == 22 {
        custom_ell_instruction_D_1(
            // For p_1
            &mut account_main.coeff_2_range,
            &mut account_main.coeff_1_range,
            &mut account_main.coeff_0_range,
            &account_main.p_3_y_range,
            &account_main.p_3_x_range,
        );

        account_main.changed_variables[COEFF_2_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_1_RANGE_INDEX] = true;
        account_main.changed_variables[COEFF_0_RANGE_INDEX] = true;
    } else if id == 23 {
        addition_step_helper::<ark_bn254::Parameters>(
            &mut account_main.proof_b,
            &account_main.proof_b_tmp_range,
            "normal",
        );
        account_main.changed_variables[PROOF_B_INDEX] = true;
    } else if id == 24 {
        addition_step_helper::<ark_bn254::Parameters>(
            &mut account_main.proof_b,
            &account_main.proof_b_tmp_range,
            "negq",
        );
        account_main.changed_variables[PROOF_B_INDEX] = true;
    } else if id == 25 {
        addition_step_helper::<ark_bn254::Parameters>(
            &mut account_main.proof_b,
            &account_main.proof_b_tmp_range,
            "q1",
        );
        account_main.changed_variables[PROOF_B_INDEX] = true;
    } else if id == 26 {
        addition_step_helper::<ark_bn254::Parameters>(
            &mut account_main.proof_b,
            &account_main.proof_b_tmp_range,
            "q2",
        );
        account_main.changed_variables[PROOF_B_INDEX] = true;
    } else if id == 69 {
        ell_instruction_d(
            &mut account_main.f_range,
            &account_main.coeff_0_range,
            &account_main.coeff_1_range,
            &account_main.coeff_2_range,
            &account_main.p_1_y_range,
            &account_main.p_1_x_range,
        );
        account_main.changed_variables[F_RANGE_INDEX] = true;
    } else if id == 70 {
        ell_instruction_d_c2(
            &mut account_main.f_range,
            &account_main.p_2_y_range,
            &account_main.p_2_x_range,
            &mut account_main.current_coeff_2_range,
        );
        account_main.changed_variables[F_RANGE_INDEX] = true;
        account_main.changed_variables[CURRENT_COEFF_2_RANGE_INDEX] = true;
    } else if id == 71 {
        ell_instruction_d_c3(
            &mut account_main.f_range,
            &account_main.p_3_y_range,
            &account_main.p_3_x_range,
            &mut account_main.current_coeff_3_range,
        );
        account_main.changed_variables[F_RANGE_INDEX] = true;
        account_main.changed_variables[CURRENT_COEFF_3_RANGE_INDEX] = true;
    } else if id == 72 {
        // test: replaces 3, 4
        square_in_place_instruction(&mut account_main.f_range);
        account_main.changed_variables[F_RANGE_INDEX] = true;
    } else if id == 73 {
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
    } else if id == 74 {
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
    } else if id == 75 {
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
    } else if id == 76 {
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
    } else if id == 77 {
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
