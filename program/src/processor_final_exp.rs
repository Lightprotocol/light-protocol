use crate::ranges_part_2::*;
use crate::state_final_exp::{FinalExpBytes};
use crate::state_merkle_tree;

use crate::instructions_final_exponentiation::{
    custom_frobenius_map_1_1,
    custom_frobenius_map_1_2,
    custom_frobenius_map_2_1,
    custom_frobenius_map_2_2,
    custom_frobenius_map_3_1,
    custom_frobenius_map_3_2,
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
    mul_assign_1,
    mul_assign_2,
    mul_assign_3,
    mul_assign_4_1,
    mul_assign_4_2,
    mul_assign_5,
    mul_assign_1_2,
    mul_assign_3_4_5,
    custom_cyclotomic_square_in_place,
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
            account_struct.f_f2_range_s = account_struct.f1_r_range_s.clone();
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
            mul_assign_1(
				&account_struct.f1_r_range_s,  f_cubic_0_range,
				&account_struct.f_f2_range_s,  f_cubic_0_range,
				&mut account_struct.cubic_range_0_s, solo_cubic_0_range
			);
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 11 {
            mul_assign_2(
				&account_struct.f1_r_range_s,  f_cubic_1_range,
				&account_struct.f_f2_range_s,  f_cubic_1_range,
				&mut account_struct.cubic_range_1_s,  solo_cubic_0_range
			);
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 12 {
            mul_assign_3(
				&mut account_struct.f1_r_range_s
			);
            account_struct.changed_variables[f1_r_range_iter] = true;

        } else if id == 13 {
            mul_assign_4_1(
				&account_struct.f_f2_range_s,
				&mut account_struct.cubic_range_2_s,
			);
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 14 {
            mul_assign_4_2(
				&mut account_struct.f1_r_range_s,
				 f_cubic_1_range,
				&account_struct.cubic_range_2_s,
			);
            account_struct.changed_variables[f1_r_range_iter] = true;

        } else if id == 15 {
            mul_assign_5(
				&mut account_struct.f1_r_range_s,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s
			);
            account_struct.changed_variables[f1_r_range_iter] = true;

        } else if id == 16 {
            account_struct.f_f2_range_s = account_struct.f1_r_range_s.clone();
            account_struct.changed_variables[f_f2_range_iter] = true;

        } else if id == 17 {
            custom_frobenius_map_2_1(&mut account_struct.f1_r_range_s);
            account_struct.changed_variables[f1_r_range_iter] = true;

        } else if id == 18 {
            custom_frobenius_map_2_2(&mut account_struct.f1_r_range_s);
            account_struct.changed_variables[f1_r_range_iter] = true;

        } else if id == 19 {
            account_struct.i_range_s = account_struct.f1_r_range_s.clone();
            conjugate_wrapper(&mut account_struct.i_range_s);
            account_struct.y0_range_s = account_struct.f1_r_range_s.clone();
            account_struct.changed_variables[i_range_iter] = true;
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 20 {
            custom_cyclotomic_square_in_place(&mut account_struct.y0_range_s);
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 21 {
            mul_assign_1(
                &account_struct.y0_range_s, f_cubic_0_range,
                &account_struct.f1_r_range_s, f_cubic_0_range,
                &mut account_struct.cubic_range_0_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 22 {
            mul_assign_2(
                &account_struct.y0_range_s, f_cubic_1_range,
                &account_struct.f1_r_range_s, f_cubic_1_range,
                &mut account_struct.cubic_range_1_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 23 {
            mul_assign_3(
                &mut account_struct.y0_range_s
            );
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 24 {
            mul_assign_4_1(
                &account_struct.f1_r_range_s,
                &mut account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 25 {
            mul_assign_4_2(
                &mut account_struct.y0_range_s,
                f_cubic_1_range,
                &account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 26 {
            mul_assign_5(
                &mut account_struct.y0_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 27 {
            mul_assign_1(
                &account_struct.y0_range_s, f_cubic_0_range,
                &account_struct.i_range_s, f_cubic_0_range,
                &mut account_struct.cubic_range_0_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 28 {
            mul_assign_2(
                &account_struct.y0_range_s, f_cubic_1_range,
                &account_struct.i_range_s, f_cubic_1_range,
                &mut account_struct.cubic_range_1_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 29 {
            mul_assign_3(
                &mut account_struct.y0_range_s
            );
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 30 {
            mul_assign_4_1(
                &account_struct.i_range_s,
                &mut account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 31 {
            mul_assign_4_2(
                &mut account_struct.y0_range_s,
                f_cubic_1_range,
                &account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 32 {
            mul_assign_5(
                &mut account_struct.y0_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[y0_range_iter] = true;


        } else if id == 33 {
            conjugate_wrapper(&mut account_struct.y0_range_s);
            custom_cyclotomic_square(&account_struct.y0_range_s, &mut account_struct.y1_range_s);
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 34 {
            custom_cyclotomic_square(&account_struct.y1_range_s , &mut account_struct.y0_range_s);
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 35 {
            mul_assign_1(
				&account_struct.y0_range_s,  f_cubic_0_range,
				&account_struct.y1_range_s,  f_cubic_0_range,
				&mut account_struct.cubic_range_0_s,  solo_cubic_0_range
			);
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 36 {
            mul_assign_2(
                &account_struct.y0_range_s,  f_cubic_1_range,
                &account_struct.y1_range_s,  f_cubic_1_range,
                &mut account_struct.cubic_range_1_s,  solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 37 {
            mul_assign_3(
                &mut account_struct.y0_range_s
            );
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 38 {
            mul_assign_4_1(
                &account_struct.y1_range_s,
                &mut account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 39 {
            mul_assign_4_2(
                &mut account_struct.y0_range_s,
                 f_cubic_1_range,
                &account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 40 {
            mul_assign_5(
                &mut account_struct.y0_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 41 {
            account_struct.i_range_s = account_struct.y0_range_s.clone();
            conjugate_wrapper(&mut account_struct.i_range_s);
            account_struct.y2_range_s = account_struct.y0_range_s.clone();
            account_struct.changed_variables[i_range_iter] = true;
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 42 {
            custom_cyclotomic_square_in_place(&mut account_struct.y2_range_s);
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 43 {
            mul_assign_1(
                &account_struct.y2_range_s, f_cubic_0_range,
                &account_struct.y0_range_s, f_cubic_0_range,
                &mut account_struct.cubic_range_0_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 44 {
            mul_assign_2(
                &account_struct.y2_range_s, f_cubic_1_range,
                &account_struct.y0_range_s, f_cubic_1_range,
                &mut account_struct.cubic_range_1_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 45 {
            mul_assign_3(
                &mut account_struct.y2_range_s
            );
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 46 {
            mul_assign_4_1(
                &account_struct.y0_range_s,
                &mut account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 47 {
            mul_assign_4_2(
                &mut account_struct.y2_range_s,
                f_cubic_1_range,
                &account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 48 {
            mul_assign_5(
                &mut account_struct.y2_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 49 {
            mul_assign_1(
                &account_struct.y2_range_s, f_cubic_0_range,
                &account_struct.i_range_s, f_cubic_0_range,
                &mut account_struct.cubic_range_0_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 50 {
            mul_assign_2(
                &account_struct.y2_range_s, f_cubic_1_range,
                &account_struct.i_range_s, f_cubic_1_range,
                &mut account_struct.cubic_range_1_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 51 {
            mul_assign_3(
                &mut account_struct.y2_range_s
            );
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 52 {
            mul_assign_4_1(
                &account_struct.i_range_s,
                &mut account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 53 {
            mul_assign_4_2(
                &mut account_struct.y2_range_s,
                f_cubic_1_range,
                &account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 54 {
            mul_assign_5(
                &mut account_struct.y2_range_s,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 55 {
            conjugate_wrapper(&mut account_struct.y2_range_s);
            custom_cyclotomic_square(&account_struct.y2_range_s, &mut account_struct.f_f2_range_s);
            account_struct.changed_variables[y2_range_iter] = true;
            account_struct.changed_variables[f_f2_range_iter] = true;

        } else if id == 56 {
            account_struct.i_range_s = account_struct.f_f2_range_s.clone();
            conjugate_wrapper(&mut account_struct.i_range_s);
            account_struct.y6_range = account_struct.f_f2_range_s.clone();
            //custom_cyclotomic_square_in_place(&mut account_struct.y6_range);
            account_struct.changed_variables[i_range_iter] = true;
            account_struct.changed_variables[y6_range_iter] = true;

        } else if id == 57 {
            custom_cyclotomic_square_in_place(&mut account_struct.y6_range);
            account_struct.changed_variables[y6_range_iter] = true;

        } else if id == 58 {
            mul_assign_1(
                &account_struct.y6_range, f_cubic_0_range,
                &account_struct.f_f2_range_s, f_cubic_0_range,
                &mut account_struct.cubic_range_0_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 59 {
            mul_assign_2(
                &account_struct.y6_range, f_cubic_1_range,
                &account_struct.f_f2_range_s, f_cubic_1_range,
                &mut account_struct.cubic_range_1_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 60 {
            mul_assign_3(
                &mut account_struct.y6_range
            );
            account_struct.changed_variables[y6_range_iter] = true;

        } else if id == 61 {
            mul_assign_4_1(
                &account_struct.f_f2_range_s,
                &mut account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 62 {
            mul_assign_4_2(
                &mut account_struct.y6_range,
                f_cubic_1_range,
                &account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[y6_range_iter] = true;

        } else if id == 63 {
            mul_assign_5(
                &mut account_struct.y6_range,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[y6_range_iter] = true;

        } else if id == 64 {
            mul_assign_1(
                &account_struct.y6_range, f_cubic_0_range,
                &account_struct.i_range_s, f_cubic_0_range,
                &mut account_struct.cubic_range_0_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 65 {
            mul_assign_2(
                &account_struct.y6_range, f_cubic_1_range,
                &account_struct.i_range_s, f_cubic_1_range,
                &mut account_struct.cubic_range_1_s, solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 66 {
            mul_assign_3(
                &mut account_struct.y6_range
            );
            account_struct.changed_variables[y6_range_iter] = true;

        } else if id == 67 {
            mul_assign_4_1(
                &account_struct.i_range_s,
                &mut account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 68 {
            mul_assign_4_2(
                &mut account_struct.y6_range,
                f_cubic_1_range,
                &account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[y6_range_iter] = true;

        } else if id == 69 {
            mul_assign_5(
                &mut account_struct.y6_range,
                &account_struct.cubic_range_0_s,
                &account_struct.cubic_range_1_s
            );
            account_struct.changed_variables[y6_range_iter] = true;

        } else if id == 70 {
            conjugate_wrapper(&mut account_struct.y6_range);
            conjugate_wrapper(&mut account_struct.y0_range_s);
            conjugate_wrapper(&mut account_struct.y6_range);
            account_struct.changed_variables[y6_range_iter] = true;
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 71 {
            mul_assign_1(
                &account_struct.y6_range,  f_cubic_0_range,
                &account_struct.y2_range_s,  f_cubic_0_range,
                &mut account_struct.cubic_range_0_s,  solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 72 {
            mul_assign_2(
                &account_struct.y6_range,  f_cubic_1_range,
                &account_struct.y2_range_s,  f_cubic_1_range,
                &mut account_struct.cubic_range_1_s,  solo_cubic_0_range
            );
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 73 {
            mul_assign_3(
                &mut account_struct.y6_range
            );
            account_struct.changed_variables[y6_range_iter] = true;

        } else if id == 74 {
            mul_assign_4_1(
                &account_struct.y2_range_s,
                &mut account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 75 {
            mul_assign_4_2(
                &mut account_struct.y6_range,
                 f_cubic_1_range,
                &account_struct.cubic_range_2_s,
            );
            account_struct.changed_variables[y6_range_iter] = true;

        } else if id == 76 {
            mul_assign_5(
				&mut account_struct.y6_range,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s
			);
            account_struct.changed_variables[y6_range_iter] = true;

        } else if id == 77 {
            mul_assign_1(
				&account_struct.y6_range,  f_cubic_0_range,
				&account_struct.y0_range_s,  f_cubic_0_range,
				&mut account_struct.cubic_range_0_s,  solo_cubic_0_range
			);
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 78 {
            mul_assign_2(
				&account_struct.y6_range,  f_cubic_1_range,
				&account_struct.y0_range_s,  f_cubic_1_range,
				&mut account_struct.cubic_range_1_s,  solo_cubic_0_range
			);
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 79 {
            mul_assign_3(
				&mut account_struct.y6_range
			);

            account_struct.changed_variables[y6_range_iter] = true;

        } else if id == 80 {
            mul_assign_4_1(
				&account_struct.y0_range_s,
				&mut account_struct.cubic_range_2_s,
			);
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 81 {
            mul_assign_4_2(
				&mut account_struct.y6_range,
				 f_cubic_1_range,
				&account_struct.cubic_range_2_s,
			);

            account_struct.changed_variables[y6_range_iter] = true;

        } else if id == 82 {
            mul_assign_5(
				&mut account_struct.y6_range,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s
			);
            account_struct.changed_variables[y6_range_iter] = true;

        } else if id == 83 {
            mul_assign_1(
				&account_struct.y1_range_s,  f_cubic_0_range,
				&account_struct.y6_range,  f_cubic_0_range,
				&mut account_struct.cubic_range_0_s,  solo_cubic_0_range
			);
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 84 {
            mul_assign_2(
				&account_struct.y1_range_s,  f_cubic_1_range,
				&account_struct.y6_range,  f_cubic_1_range,
				&mut account_struct.cubic_range_1_s,  solo_cubic_0_range
			);
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 85 {
            mul_assign_3(
				&mut account_struct.y1_range_s
			);
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 86 {
            mul_assign_4_1(
				&account_struct.y6_range,
				&mut account_struct.cubic_range_2_s,
			);
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 87 {
            mul_assign_4_2(
				&mut account_struct.y1_range_s,
				 f_cubic_1_range,
				&account_struct.cubic_range_2_s,
			);
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 88 {
            mul_assign_5(
				&mut account_struct.y1_range_s,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s
			);
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 89 {
            mul_assign_1(
				&account_struct.y2_range_s,  f_cubic_0_range,
				&account_struct.y6_range,  f_cubic_0_range,
				&mut account_struct.cubic_range_0_s,  solo_cubic_0_range
			);
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 90 {
            mul_assign_2(
				&account_struct.y2_range_s,  f_cubic_1_range,
				&account_struct.y6_range,  f_cubic_1_range,
				&mut account_struct.cubic_range_1_s,  solo_cubic_0_range
			);
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 91 {
            mul_assign_3(
				&mut account_struct.y2_range_s
			);
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 92 {
            mul_assign_4_1(
				&account_struct.y6_range,
				&mut account_struct.cubic_range_2_s,
			);
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 93 {
            mul_assign_4_2(
				&mut account_struct.y2_range_s,
				 f_cubic_1_range,
				&account_struct.cubic_range_2_s,
			);
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 94 {
            mul_assign_5(
				&mut account_struct.y2_range_s,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s
			);
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 95 {
            mul_assign_1(
				&account_struct.y2_range_s,  f_cubic_0_range,
				&account_struct.f1_r_range_s,  f_cubic_0_range,
				&mut account_struct.cubic_range_0_s,  solo_cubic_0_range
			);
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 96 {
            mul_assign_2(
				&account_struct.y2_range_s,  f_cubic_1_range,
				&account_struct.f1_r_range_s,  f_cubic_1_range,
				&mut account_struct.cubic_range_1_s,  solo_cubic_0_range
			);
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 97 {
            mul_assign_3(
				&mut account_struct.y2_range_s
			);

            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 98 {
            mul_assign_4_1(
				&account_struct.f1_r_range_s,
				&mut account_struct.cubic_range_2_s,
			);
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 99 {
            mul_assign_4_2(
				&mut account_struct.y2_range_s,
				 f_cubic_1_range,
				&account_struct.cubic_range_2_s,
			);
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 100 {
            mul_assign_5(
				&mut account_struct.y2_range_s,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s
			);
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 101 {
            account_struct.y0_range_s = account_struct.y1_range_s.clone();

            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 102 {
            custom_frobenius_map_1_1(&mut account_struct.y0_range_s);
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 103 {
            custom_frobenius_map_1_2(&mut account_struct.y0_range_s);
            account_struct.changed_variables[y0_range_iter] = true;

        } else if id == 104 {
            mul_assign_1(
				&account_struct.y2_range_s,  f_cubic_0_range,
				&account_struct.y0_range_s,  f_cubic_0_range,
				&mut account_struct.cubic_range_0_s,  solo_cubic_0_range
			);
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 105 {
            mul_assign_2(
				&account_struct.y2_range_s,  f_cubic_1_range,
				&account_struct.y0_range_s,  f_cubic_1_range,
				&mut account_struct.cubic_range_1_s,  solo_cubic_0_range
			);
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 106 {
            mul_assign_3(
				&mut account_struct.y2_range_s
			);
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 107 {
            mul_assign_4_1(
				&account_struct.y0_range_s,
				&mut account_struct.cubic_range_2_s,
			);
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 108 {
            mul_assign_4_2(
				&mut account_struct.y2_range_s,
				 f_cubic_1_range,
				&account_struct.cubic_range_2_s,
			);
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 109 {
            mul_assign_5(
				&mut account_struct.y2_range_s,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s
			);
            account_struct.changed_variables[y2_range_iter] = true;

        } else if id == 110 {
            custom_frobenius_map_2_1(&mut account_struct.y6_range);
            account_struct.changed_variables[y6_range_iter] = true;

        } else if id == 111 {
            custom_frobenius_map_2_2(&mut account_struct.y6_range);
            account_struct.changed_variables[y6_range_iter] = true;

        } else if id == 112 {
            conjugate_wrapper(&mut account_struct.f1_r_range_s);

            account_struct.changed_variables[f1_r_range_iter] = true;

        } else if id == 113 {
            mul_assign_1(
				&account_struct.y1_range_s,  f_cubic_0_range,
				&account_struct.f1_r_range_s,  f_cubic_0_range,
				&mut account_struct.cubic_range_0_s,  solo_cubic_0_range
			);
            account_struct.changed_variables[cubic_range_0_iter] = true;

        } else if id == 114 {
            mul_assign_2(
				&account_struct.y1_range_s,  f_cubic_1_range,
				&account_struct.f1_r_range_s,  f_cubic_1_range,
				&mut account_struct.cubic_range_1_s,  solo_cubic_0_range
			);
            account_struct.changed_variables[cubic_range_1_iter] = true;

        } else if id == 115 {
            mul_assign_3(
				&mut account_struct.y1_range_s
			);
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 116 {
            mul_assign_4_1(
				&account_struct.f1_r_range_s,
				&mut account_struct.cubic_range_2_s,
			);
            account_struct.changed_variables[cubic_range_2_iter] = true;

        } else if id == 117 {
            mul_assign_4_2(
				&mut account_struct.y1_range_s,
				 f_cubic_1_range,
				&account_struct.cubic_range_2_s,
			);
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 118 {
            mul_assign_5(
				&mut account_struct.y1_range_s,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s
			);
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 119 {
            custom_frobenius_map_3_1(&mut account_struct.y1_range_s);
            account_struct.changed_variables[y1_range_iter] = true;

        } else if id == 120 {
            custom_frobenius_map_3_2(&mut account_struct.y1_range_s);
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
}
