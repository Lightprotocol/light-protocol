use crate::instructions_prepare_inputs::*;
use crate::ranges_prepare_inputs::*;
// use crate::constraints::{BytesCoeffs, BytesMain};
use crate::PrepareInputsBytes;

// get alg
pub fn _process_instruction_prepare_inputs(
    id: u8,
    account: &mut PrepareInputsBytes,
    // coeff_0: &Vec<u8>,
    public_inputs: &[ark_ff::Fp256<ark_ed_on_bls12_381::FqParameters>],
    current_index: usize,
) {
    // i_order: [0,1,256*2,6,    1,256*3,6, .... x4]
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
            &mut account.g_ic_x_range,
            &mut account.g_ic_y_range,
            &mut account.g_ic_z_range,
        );
        account.changed_variables[I_1_RANGE_INDEX] = true;
        account.changed_variables[X_1_RANGE_INDEX] = true;
        account.changed_variables[I_2_RANGE_INDEX] = true;
        account.changed_variables[X_2_RANGE_INDEX] = true;
        account.changed_variables[I_3_RANGE_INDEX] = true;
        account.changed_variables[X_3_RANGE_INDEX] = true;
        account.changed_variables[I_4_RANGE_INDEX] = true;
        account.changed_variables[X_4_RANGE_INDEX] = true;

        account.changed_variables[G_IC_X_RANGE_INDEX] = true;
        account.changed_variables[G_IC_Y_RANGE_INDEX] = true;
        account.changed_variables[G_IC_Z_RANGE_INDEX] = true;
    } else if id == 41 {
        init_res_instruction(
            &mut account.res_x_range,
            &mut account.res_y_range,
            &mut account.res_z_range,
        );
        account.changed_variables[RES_X_RANGE_INDEX] = true;
        account.changed_variables[RES_Y_RANGE_INDEX] = true;
        account.changed_variables[RES_Z_RANGE_INDEX] = true;
    } else if id == 42 {
        maths_instruction(
            &mut account.res_x_range,
            &mut account.res_y_range,
            &mut account.res_z_range,
            &account.i_1_range,
            &account.x_1_range,
            current_index,
        ); // 1 of 256
        account.changed_variables[RES_X_RANGE_INDEX] = true;
        account.changed_variables[RES_Y_RANGE_INDEX] = true;
        account.changed_variables[RES_Z_RANGE_INDEX] = true;
        account.changed_variables[I_1_RANGE_INDEX] = true;
        account.changed_variables[X_1_RANGE_INDEX] = true;
    } else if id == 43 {
        maths_instruction(
            &mut account.res_x_range,
            &mut account.res_y_range,
            &mut account.res_z_range,
            &account.i_2_range,
            &account.x_2_range,
            current_index,
        );
        account.changed_variables[RES_X_RANGE_INDEX] = true;
        account.changed_variables[RES_Y_RANGE_INDEX] = true;
        account.changed_variables[RES_Z_RANGE_INDEX] = true;
        account.changed_variables[I_2_RANGE_INDEX] = true;
        account.changed_variables[X_2_RANGE_INDEX] = true;
    } else if id == 44 {
        maths_instruction(
            &mut account.res_x_range,
            &mut account.res_y_range,
            &mut account.res_z_range,
            &account.i_3_range,
            &account.x_3_range,
            current_index,
        );
        account.changed_variables[RES_X_RANGE_INDEX] = true;
        account.changed_variables[RES_Y_RANGE_INDEX] = true;
        account.changed_variables[RES_Z_RANGE_INDEX] = true;
        account.changed_variables[I_3_RANGE_INDEX] = true;
        account.changed_variables[X_3_RANGE_INDEX] = true;
    } else if id == 45 {
        maths_instruction(
            &mut account.res_x_range,
            &mut account.res_y_range,
            &mut account.res_z_range,
            &account.i_4_range,
            &account.x_4_range,
            current_index,
        );
        account.changed_variables[RES_X_RANGE_INDEX] = true;
        account.changed_variables[RES_Y_RANGE_INDEX] = true;
        account.changed_variables[RES_Z_RANGE_INDEX] = true;
        account.changed_variables[I_4_RANGE_INDEX] = true;
        account.changed_variables[X_4_RANGE_INDEX] = true;
    } else if id == 46 {
        maths_g_ic_instruction(
            &mut account.g_ic_x_range,
            &mut account.g_ic_y_range,
            &mut account.g_ic_z_range,
            &account.res_x_range,
            &account.res_y_range,
            &account.res_z_range,
        );
        account.changed_variables[G_IC_X_RANGE_INDEX] = true;
        account.changed_variables[G_IC_Y_RANGE_INDEX] = true;
        account.changed_variables[G_IC_Z_RANGE_INDEX] = true;
        account.changed_variables[RES_X_RANGE_INDEX] = true;
        account.changed_variables[RES_Y_RANGE_INDEX] = true;
        account.changed_variables[RES_Z_RANGE_INDEX] = true;
    } // last: acc_main_p2 = g_ic
}
