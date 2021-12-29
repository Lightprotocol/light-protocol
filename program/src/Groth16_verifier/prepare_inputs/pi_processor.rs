use crate::Groth16_verifier::{
    prepare_inputs::{
        pi_state::PiBytes,
        pi_ranges::*,
        pi_instructions::*,
    }
};
use solana_program::{log, msg};

pub fn _pi_process_instruction(
    id: u8,
    account: &mut PiBytes,
    //public_inputs: &[ark_ff::Fp256<ark_ed_on_bn254::FqParameters>],
    current_index: usize,
) {
    // i_order: [0,1,256*2,6,    1,256*3,6, .... x4]
    msg!("instruction: {:?}", id);
    log::sol_log_compute_units();
    /*
    if id == 40 {
        init_pairs_instruction(
            &public_inputs,
            &mut account.i_1_range,
            &mut account.x_1_range,
            &mut account.i_2_range,
            &mut account.x_2_range,
            &mut account.i_3_range,
            &mut account.x_3_range,
            &mut account.i_4_range,
            &mut account.x_4_range,
            &mut account.i_5_range,
            &mut account.x_5_range,
            &mut account.i_6_range,
            &mut account.x_6_range,
            &mut account.i_7_range,
            &mut account.x_7_range,
            &mut account.g_ic_x_range,
            &mut account.g_ic_y_range,
            &mut account.g_ic_z_range,
        );

        let indices: [usize; 17] = [
            I_1_RANGE_INDEX,
            X_1_RANGE_INDEX,
            I_2_RANGE_INDEX,
            X_2_RANGE_INDEX,
            I_3_RANGE_INDEX,
            X_3_RANGE_INDEX,
            I_4_RANGE_INDEX,
            X_4_RANGE_INDEX,
            I_5_RANGE_INDEX,
            X_5_RANGE_INDEX,
            I_6_RANGE_INDEX,
            X_6_RANGE_INDEX,
            I_7_RANGE_INDEX,
            X_7_RANGE_INDEX,
            G_IC_X_RANGE_INDEX,
            G_IC_Y_RANGE_INDEX,
            G_IC_Z_RANGE_INDEX,
        ];
        for i in indices.iter() {
            account.changed_variables[*i] = true;
        }
    } else */if id == 41 {
        init_res_instruction(
            &mut account.res_x_range,
            &mut account.res_y_range,
            &mut account.res_z_range,
        );
        let indices = [RES_X_RANGE_INDEX, RES_Y_RANGE_INDEX, RES_Z_RANGE_INDEX];
        for i in indices.iter() {
            account.changed_variables[*i] = true;
        }
    } else if id == 42 {
        maths_instruction(
            &mut account.res_x_range,
            &mut account.res_y_range,
            &mut account.res_z_range,
            &account.i_1_range,
            &account.x_1_range,
            current_index,
        ); // 1 of 256
        let indices = [RES_X_RANGE_INDEX, RES_Y_RANGE_INDEX, RES_Z_RANGE_INDEX];
        for i in indices.iter() {
            account.changed_variables[*i] = true;
        }
    } else if id == 43 {
        maths_instruction(
            &mut account.res_x_range,
            &mut account.res_y_range,
            &mut account.res_z_range,
            &account.i_2_range,
            &account.x_2_range,
            current_index,
        );
        let indices = [RES_X_RANGE_INDEX, RES_Y_RANGE_INDEX, RES_Z_RANGE_INDEX];
        for i in indices.iter() {
            account.changed_variables[*i] = true;
        }
    } else if id == 44 {
        maths_instruction(
            &mut account.res_x_range,
            &mut account.res_y_range,
            &mut account.res_z_range,
            &account.i_3_range,
            &account.x_3_range,
            current_index,
        );
        let indices = [RES_X_RANGE_INDEX, RES_Y_RANGE_INDEX, RES_Z_RANGE_INDEX];
        for i in indices.iter() {
            account.changed_variables[*i] = true;
        }
    } else if id == 45 {
        maths_instruction(
            &mut account.res_x_range,
            &mut account.res_y_range,
            &mut account.res_z_range,
            &account.i_4_range,
            &account.x_4_range,
            current_index,
        );
        let indices = [RES_X_RANGE_INDEX, RES_Y_RANGE_INDEX, RES_Z_RANGE_INDEX];
        for i in indices.iter() {
            account.changed_variables[*i] = true;
        }
    } else if id == 46 {
        maths_g_ic_instruction(
            &mut account.g_ic_x_range,
            &mut account.g_ic_y_range,
            &mut account.g_ic_z_range,
            &account.res_x_range,
            &account.res_y_range,
            &account.res_z_range,
        );
        let indices = [G_IC_X_RANGE_INDEX, G_IC_Y_RANGE_INDEX, G_IC_Z_RANGE_INDEX];
        for i in indices.iter() {
            account.changed_variables[*i] = true;
        }
    } else if id == 47 {
        // migrated from preprocessor
        g_ic_into_affine_1(
            &mut account.g_ic_x_range,
            &mut account.g_ic_y_range,
            &mut account.g_ic_z_range, // only one changing
        );
        let indices = [G_IC_X_RANGE_INDEX, G_IC_Y_RANGE_INDEX, G_IC_Z_RANGE_INDEX];
        for i in indices.iter() {
            account.changed_variables[*i] = true;
        }
    } else if id == 48 {
        // migrated from preprocessor
        g_ic_into_affine_2(
            &account.g_ic_x_range,
            &account.g_ic_y_range,
            &account.g_ic_z_range,
            &mut account.x_1_range,
        );
        let indices = [X_1_RANGE_INDEX];
        for i in indices.iter() {
            account.changed_variables[*i] = true;
        }
    } else if id == 56 {
        maths_instruction(
            &mut account.res_x_range,
            &mut account.res_y_range,
            &mut account.res_z_range,
            &account.i_5_range,
            &account.x_5_range,
            current_index,
        );
        let indices = [RES_X_RANGE_INDEX, RES_Y_RANGE_INDEX, RES_Z_RANGE_INDEX];
        for i in indices.iter() {
            account.changed_variables[*i] = true;
        }
    } else if id == 57 {
        maths_instruction(
            &mut account.res_x_range,
            &mut account.res_y_range,
            &mut account.res_z_range,
            &account.i_6_range,
            &account.x_6_range,
            current_index,
        );
        let indices = [RES_X_RANGE_INDEX, RES_Y_RANGE_INDEX, RES_Z_RANGE_INDEX];
        for i in indices.iter() {
            account.changed_variables[*i] = true;
        }
    } else if id == 58 {
        maths_instruction(
            &mut account.res_x_range,
            &mut account.res_y_range,
            &mut account.res_z_range,
            &account.i_7_range,
            &account.x_7_range,
            current_index,
        );
        let indices = [RES_X_RANGE_INDEX, RES_Y_RANGE_INDEX, RES_Z_RANGE_INDEX];
        for i in indices.iter() {
            account.changed_variables[*i] = true;
        }
    }
    msg!("executed, cost: ");
    log::sol_log_compute_units();
}
