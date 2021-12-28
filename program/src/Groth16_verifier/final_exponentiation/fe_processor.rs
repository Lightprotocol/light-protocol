use crate::Groth16_verifier::final_exponentiation::{
    fe_ranges::*,
    fe_state::FinalExpBytes,
    fe_instructions::{
        custom_frobenius_map_1,
        custom_frobenius_map_2,
        custom_frobenius_map_3,
        custom_cyclotomic_square,
        conjugate_wrapper,
        custom_f_inverse_1,
        custom_f_inverse_2,
        custom_f_inverse_3,
        custom_f_inverse_4,
        custom_f_inverse_5,
        custom_cubic_inverse_1,
        custom_cubic_inverse_2,
        custom_quadratic_fp256_inverse_1,
        custom_quadratic_fp256_inverse_2,
        mul_assign_1_2,
        mul_assign_3_4_5,
        custom_cyclotomic_square_in_place,
    },
};

use solana_program::{
    msg,
    log::sol_log_compute_units,
    account_info::{next_account_info, AccountInfo},
    program_error::ProgramError,
    program_pack::Pack,
};

pub fn _process_instruction_final_exp(
        account_struct: &mut FinalExpBytes,
        id: u8
    ) {

        if id == 0 {
            //init and conjugate
            account_struct.f1_r_range_s = account_struct.f_f2_range_s.clone();
            //account_struct.f_f2_range_s = account_struct.f1_r_range_s.clone();
            conjugate_wrapper(&mut account_struct.f1_r_range_s);
            account_struct.changed_variables[f1_r_range_iter] = true;
            account_struct.changed_variables[f_f2_range_iter] = true;


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
            custom_quadratic_fp256_inverse_1(
                &account_struct.quad_range_3_s,
                &mut account_struct.fp384_range_s
            );
            account_struct.changed_variables[fp384_range_iter] = true;

        } else if id == 5 {
            custom_quadratic_fp256_inverse_2(
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
            custom_f_inverse_4(
                &mut account_struct.cubic_range_0_s,
                &account_struct.f_f2_range_s
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 9 {
            custom_f_inverse_5(
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s,
                &mut account_struct.f_f2_range_s,
            );
            account_struct.changed_variables[f_f2_range_iter] = true;

        } else if id == 10 {
            mul_assign_1_2(
                &account_struct.f1_r_range_s,
                &account_struct.f_f2_range_s,
                &mut account_struct.cubic_range_0_s,
                &mut account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 11 {
            mul_assign_3_4_5(
                &account_struct.f_f2_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s,
                &mut account_struct.f1_r_range_s,
            );
            account_struct.changed_variables[f1_r_range_iter] = true;

        } else if id == 12 {
            account_struct.f_f2_range_s = account_struct.f1_r_range_s.clone();
            account_struct.changed_variables[f_f2_range_iter] = true;

        } else if id == 13 {
            custom_frobenius_map_2(&mut account_struct.f1_r_range_s);
            account_struct.changed_variables[f1_r_range_iter] = true;

        } else if id == 14 {
            account_struct.i_range_s = account_struct.f1_r_range_s.clone();
            conjugate_wrapper(&mut account_struct.i_range_s);
            account_struct.y0_range_s = account_struct.f1_r_range_s.clone();
            account_struct.changed_variables[i_range_iter] = true;
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 15 {
            custom_cyclotomic_square_in_place(&mut account_struct.y0_range_s);
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 16 {
            mul_assign_1_2(
                &account_struct.y0_range_s,
                &account_struct.f1_r_range_s,
                &mut account_struct.cubic_range_0_s,
                &mut account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 17 {
            mul_assign_3_4_5(
                &account_struct.f1_r_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s,
                &mut account_struct.y0_range_s,
            );
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 18 {
            mul_assign_1_2(
                &account_struct.y0_range_s,
                &account_struct.i_range_s,
                &mut account_struct.cubic_range_0_s,
                &mut account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 19 {
            mul_assign_3_4_5(
                &account_struct.i_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s,
                &mut account_struct.y0_range_s,
            );
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 20 {
            conjugate_wrapper(&mut account_struct.y0_range_s);
            custom_cyclotomic_square(&account_struct.y0_range_s, &mut account_struct.y1_range_s);
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 21 {
            custom_cyclotomic_square(&account_struct.y1_range_s , &mut account_struct.y0_range_s);
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 22 {
            mul_assign_1_2(
                &account_struct.y0_range_s,
                &account_struct.y1_range_s,
                &mut account_struct.cubic_range_0_s,
                &mut account_struct.cubic_range_1_s
            );

            account_struct.changed_variables[cubic_range_0_iter] = true;
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 23 {
            mul_assign_3_4_5(
                &account_struct.y1_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s,
                &mut account_struct.y0_range_s,
            );
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 24 {
            account_struct.i_range_s = account_struct.y0_range_s.clone();
            conjugate_wrapper(&mut account_struct.i_range_s);
            account_struct.y2_range_s = account_struct.y0_range_s.clone();
            account_struct.changed_variables[i_range_iter] = true;
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 25 {
            custom_cyclotomic_square_in_place(&mut account_struct.y2_range_s);
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 26 {
            mul_assign_1_2(
                &account_struct.y2_range_s,
                &account_struct.y0_range_s,
                &mut account_struct.cubic_range_0_s,
                &mut account_struct.cubic_range_1_s
            );

            account_struct.changed_variables[cubic_range_0_iter] = true;
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 27 {
            mul_assign_3_4_5(
                &account_struct.y0_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s,
                &mut account_struct.y2_range_s,
            );

            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 28 {
            mul_assign_1_2(
                &account_struct.y2_range_s,
                &account_struct.i_range_s,
                &mut account_struct.cubic_range_0_s,
                &mut account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 29 {
            mul_assign_3_4_5(
                &account_struct.i_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s,
                &mut account_struct.y2_range_s,
            );

            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 30 {
            conjugate_wrapper(&mut account_struct.y2_range_s);
            custom_cyclotomic_square(&account_struct.y2_range_s, &mut account_struct.f_f2_range_s);
            account_struct.changed_variables[y2_range_iter] = true;
            account_struct.changed_variables[f_f2_range_iter] = true;

        } else if id == 31 {
            account_struct.i_range_s = account_struct.f_f2_range_s.clone();
            conjugate_wrapper(&mut account_struct.i_range_s);
            account_struct.y6_range = account_struct.f_f2_range_s.clone();
            //custom_cyclotomic_square_in_place(&mut account_struct.y6_range);
            account_struct.changed_variables[i_range_iter] = true;
            account_struct.changed_variables[y6_range_iter] = true;

        } else if id == 32 {
            custom_cyclotomic_square_in_place(&mut account_struct.y6_range);
            account_struct.changed_variables[y6_range_iter] = true;

        } else if id == 33 {
            mul_assign_1_2(
                &account_struct.y6_range,
                &account_struct.f_f2_range_s,
                &mut account_struct.cubic_range_0_s,
                &mut account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 34 {
            mul_assign_3_4_5(
                &account_struct.f_f2_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s,
                &mut account_struct.y6_range,
            );
            account_struct.changed_variables[y6_range_iter] = true;

        } else if id == 35 {
            mul_assign_1_2(
                &account_struct.y6_range,
                &account_struct.i_range_s,
                &mut account_struct.cubic_range_0_s,
                &mut account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 36 {
            mul_assign_3_4_5(
                &account_struct.i_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s,
                &mut account_struct.y6_range,
            );
            account_struct.changed_variables[y6_range_iter] = true;

        } else if id == 37 {
            conjugate_wrapper(&mut account_struct.y6_range);
            conjugate_wrapper(&mut account_struct.y0_range_s);
            conjugate_wrapper(&mut account_struct.y6_range);
            account_struct.changed_variables[y6_range_iter] = true;
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 38 {
            mul_assign_1_2(
                &account_struct.y6_range,
                &account_struct.y2_range_s,
                &mut account_struct.cubic_range_0_s,
                &mut account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 39 {
            mul_assign_3_4_5(
                &account_struct.y2_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s,
                &mut account_struct.y6_range,
            );
            account_struct.changed_variables[y6_range_iter] = true;

        } else if id == 40 {
            mul_assign_1_2(
                &account_struct.y6_range,
                &account_struct.y0_range_s,
                &mut account_struct.cubic_range_0_s,
                &mut account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 41 {
            mul_assign_3_4_5(
                &account_struct.y0_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s,
                &mut account_struct.y6_range,
            );
            account_struct.changed_variables[y6_range_iter] = true;

        } else if id == 42 {
            mul_assign_1_2(
                &account_struct.y1_range_s,
                &account_struct.y6_range,
                &mut account_struct.cubic_range_0_s,
                &mut account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 43 {
            mul_assign_3_4_5(
                &account_struct.y6_range,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s,
                &mut account_struct.y1_range_s,
            );
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 44 {
            mul_assign_1_2(
                &account_struct.y2_range_s,
                &account_struct.y6_range,
                &mut account_struct.cubic_range_0_s,
                &mut account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 45 {
            mul_assign_3_4_5(
                &account_struct.y6_range,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s,
                &mut account_struct.y2_range_s,
            );
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 46 {
            mul_assign_1_2(
                &account_struct.y2_range_s,
                &account_struct.f1_r_range_s,
                &mut account_struct.cubic_range_0_s,
                &mut account_struct.cubic_range_1_s
            );

            account_struct.changed_variables[cubic_range_0_iter] = true;
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 47 {
            mul_assign_3_4_5(
                &account_struct.f1_r_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s,
                &mut account_struct.y2_range_s,
            );

            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 48 {
            account_struct.y0_range_s = account_struct.y1_range_s.clone();
            custom_frobenius_map_1(&mut account_struct.y0_range_s);
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 49 {
            mul_assign_1_2(
                &account_struct.y2_range_s,
                &account_struct.y0_range_s,
                &mut account_struct.cubic_range_0_s,
                &mut account_struct.cubic_range_1_s
            );

            account_struct.changed_variables[cubic_range_0_iter] = true;
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 50 {
            mul_assign_3_4_5(
                &account_struct.y0_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s,
                &mut account_struct.y2_range_s,
            );
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 51 {
            custom_frobenius_map_2(&mut account_struct.y6_range);
            account_struct.changed_variables[y6_range_iter] = true;

        } else if id == 52 {
            conjugate_wrapper(&mut account_struct.f1_r_range_s);

            account_struct.changed_variables[f1_r_range_iter] = true;

        } else if id == 53 {
            mul_assign_1_2(
                &account_struct.y1_range_s,
                &account_struct.f1_r_range_s,
                &mut account_struct.cubic_range_0_s,
                &mut account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;
            account_struct.changed_variables[cubic_range_1_iter] = true;
        } else if id == 54 {
            mul_assign_3_4_5(
                &account_struct.f1_r_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s,
                &mut account_struct.y1_range_s,
            );
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 55 {
            custom_frobenius_map_3(&mut account_struct.y1_range_s);
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 121 {
            //let mut actual_f = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::one();

            mul_assign_1_2(&account_struct.f1_r_range_s,
            &account_struct.f_f2_range_s,
            &mut account_struct.cubic_range_0_s,
            &mut account_struct.cubic_range_1_s);
            account_struct.changed_variables[cubic_range_0_iter] = true;
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 122 {
            mul_assign_3_4_5(&mut account_struct.f1_r_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s,
                &mut account_struct.f_f2_range_s
            );
            account_struct.changed_variables[f1_r_range_iter] = true;
        }
        println!("processor wants to modify {:?}",account_struct.changed_variables);
}
