use crate::ml_instructions::*;
use crate::ml_parsers::*;
use crate::ml_processor::*;
use crate::ml_ranges::*;
use crate::ml_state::*;
use crate::pi_state::*;
use solana_program::program_pack::Pack;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    log::sol_log_compute_units,
    msg,
    program_error::ProgramError,
};
use crate::IX_ORDER;

pub fn _pre_process_instruction_miller_loop(
    _instruction_data: &[u8],
    accounts: &[AccountInfo],
) -> Result<(), ProgramError> {
    msg!("entered _pre_process_instruction_miller_loop");

    let account = &mut accounts.iter();
    let signing_account = next_account_info(account)?;
    let account_main = next_account_info(account)?;
    msg!("unpacking");
    let mut account_main_data = ML254Bytes::unpack(&account_main.data.borrow())?;
    msg!("unpacked Current instruction {}", IX_ORDER[account_main_data.current_instruction_index]);

    // First ix: "0" -> Parses g_ic_affine from prepared_inputs.
    // Hardcoded for test purposes.
    if IX_ORDER[account_main_data.current_instruction_index] == 0 {
        msg!("parsing state from prepare inputs to ml");
        msg!("here0");
        let account_prepare_inputs_data = PiBytes::unpack(&account_main.data.borrow())?;

        let g_ic_affine = parse_x_group_affine_from_bytes(&account_prepare_inputs_data.x_1_range); // 10k
        msg!("here3");

        let p2: ark_ec::bn::G1Prepared<ark_bn254::Parameters> =
            ark_ec::bn::g1::G1Prepared::from(g_ic_affine);

        move_proofs(&mut account_main_data, &account_prepare_inputs_data);

        msg!("here4");

        parse_fp256_to_bytes(p2.0.x, &mut account_main_data.p_2_x_range);
        msg!("here5");
        parse_fp256_to_bytes(p2.0.y, &mut account_main_data.p_2_y_range);
        account_main_data.current_instruction_index += 1;

        account_main_data.changed_variables[P_2_Y_RANGE_INDEX] = true;
        account_main_data.changed_variables[P_2_X_RANGE_INDEX] = true;

        ML254Bytes::pack_into_slice(&account_main_data, &mut account_main.data.borrow_mut());
        msg!("here6");
        return Ok(());
    } else {
        // Empty vecs that pass data from the client if called with respective ix.
        _process_instruction(
            IX_ORDER[account_main_data.current_instruction_index],
            &mut account_main_data,
        );
        account_main_data.current_instruction_index += 1;

        msg!("packing");
        ML254Bytes::pack_into_slice(&account_main_data, &mut account_main.data.borrow_mut());
        msg!("packed");
        Ok(())
    }
}
