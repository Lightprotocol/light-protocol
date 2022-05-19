use crate::groth16_verifier::prepare_inputs::{
    instructions::*, ranges::*, state::PrepareInputsState,
};
use solana_program::msg;
use solana_program::program_error::ProgramError;
use std::cell::RefMut;

const ROUNDS: usize = 4 * 13;
const FILLING_ROUNDS: usize = 256 % ROUNDS;
pub fn _process_instruction(
    id: u8,
    account: &mut RefMut<'_, PrepareInputsState>,
    current_index: usize,
) -> Result<(), ProgramError> {
    // i_order: [0,1,256*2,6,    1,256*3,6, .... x7]
    msg!("instruction: {:?}", id);

    if id == 41 {
        let mut account_res_x_range = account.res_x_range;
        let mut account_res_y_range = account.res_y_range;
        let mut account_res_z_range = account.res_z_range;
        init_res_instruction(
            &mut account_res_x_range,
            &mut account_res_y_range,
            &mut account_res_z_range,
        )?;
        account.res_x_range = account_res_x_range;
        account.res_y_range = account_res_y_range;
        account.res_z_range = account_res_z_range;

    } else if id == 42 {
        let mut account_res_x_range = account.res_x_range;
        let mut account_res_y_range = account.res_y_range;
        let mut account_res_z_range = account.res_z_range;
        maths_instruction(
            &mut account_res_x_range,
            &mut account_res_y_range,
            &mut account_res_z_range,
            &account.i_1_range,
            &account.x_1_range,
            current_index,
            ROUNDS,
        )?; // 1 of 256
        account.res_x_range = account_res_x_range;
        account.res_y_range = account_res_y_range;
        account.res_z_range = account_res_z_range;
        account.current_index += (ROUNDS as u64 / 4) - 1;


    } else if id == 62 {
        // last instruction after for 256 % ROUNDS != 0
        // executes FILLING_ROUNDS rounds
        let mut account_res_x_range = account.res_x_range;
        let mut account_res_y_range = account.res_y_range;
        let mut account_res_z_range = account.res_z_range;
        maths_instruction(
            &mut account_res_x_range,
            &mut account_res_y_range,
            &mut account_res_z_range,
            &account.i_1_range,
            &account.x_1_range,
            current_index,
            FILLING_ROUNDS,
        )?; // 1 of 256
        account.current_index += 13;

        let mut account_g_ic_x_range = account.g_ic_x_range;
        let mut account_g_ic_y_range = account.g_ic_y_range;
        let mut account_g_ic_z_range = account.g_ic_z_range;
        maths_g_ic_instruction(
            &mut account_g_ic_x_range,
            &mut account_g_ic_y_range,
            &mut account_g_ic_z_range,
            &account_res_x_range,
            &account_res_y_range,
            &account_res_z_range,
        )?;
        account.g_ic_x_range = account_g_ic_x_range;
        account.g_ic_y_range = account_g_ic_y_range;
        account.g_ic_z_range = account_g_ic_z_range;

        init_res_instruction(
            &mut account_res_x_range,
            &mut account_res_y_range,
            &mut account_res_z_range,
        )?;
        account.res_x_range = account_res_x_range;
        account.res_y_range = account_res_y_range;
        account.res_z_range = account_res_z_range;

    } else if id == 43 {
        let mut account_res_x_range = account.res_x_range;
        let mut account_res_y_range = account.res_y_range;
        let mut account_res_z_range = account.res_z_range;
        maths_instruction(
            &mut account_res_x_range,
            &mut account_res_y_range,
            &mut account_res_z_range,
            &account.i_2_range,
            &account.x_2_range,
            current_index,
            ROUNDS,
        )?; // 1 of 256
        account.res_x_range = account_res_x_range;
        account.res_y_range = account_res_y_range;
        account.res_z_range = account_res_z_range;
        account.current_index += (ROUNDS as u64 / 4) - 1;


    } else if id == 63 {
        // last instruction after for 256 % ROUNDS != 0
        // executes FILLING_ROUNDS rounds
        let mut account_res_x_range = account.res_x_range;
        let mut account_res_y_range = account.res_y_range;
        let mut account_res_z_range = account.res_z_range;
        maths_instruction(
            &mut account_res_x_range,
            &mut account_res_y_range,
            &mut account_res_z_range,
            &account.i_2_range,
            &account.x_2_range,
            current_index,
            FILLING_ROUNDS,
        )?; // 1 of 256
        account.current_index += 13;


        let mut account_g_ic_x_range = account.g_ic_x_range;
        let mut account_g_ic_y_range = account.g_ic_y_range;
        let mut account_g_ic_z_range = account.g_ic_z_range;
        maths_g_ic_instruction(
            &mut account_g_ic_x_range,
            &mut account_g_ic_y_range,
            &mut account_g_ic_z_range,
            &account_res_x_range,
            &account_res_y_range,
            &account_res_z_range,
        )?;
        account.g_ic_x_range = account_g_ic_x_range;
        account.g_ic_y_range = account_g_ic_y_range;
        account.g_ic_z_range = account_g_ic_z_range;

        init_res_instruction(
            &mut account_res_x_range,
            &mut account_res_y_range,
            &mut account_res_z_range,
        )?;
        account.res_x_range = account_res_x_range;
        account.res_y_range = account_res_y_range;
        account.res_z_range = account_res_z_range;


    } else if id == 44 {
        let mut account_res_x_range = account.res_x_range;
        let mut account_res_y_range = account.res_y_range;
        let mut account_res_z_range = account.res_z_range;
        maths_instruction(
            &mut account_res_x_range,
            &mut account_res_y_range,
            &mut account_res_z_range,
            &account.i_3_range,
            &account.x_3_range,
            current_index,
            ROUNDS,
        )?; // 1 of 256
        account.res_x_range = account_res_x_range;
        account.res_y_range = account_res_y_range;
        account.res_z_range = account_res_z_range;
        account.current_index += (ROUNDS as u64 / 4) - 1;


    } else if id == 64 {
        // last instruction after for 256 % ROUNDS != 0
        // executes FILLING_ROUNDS rounds
        let mut account_res_x_range = account.res_x_range;
        let mut account_res_y_range = account.res_y_range;
        let mut account_res_z_range = account.res_z_range;
        maths_instruction(
            &mut account_res_x_range,
            &mut account_res_y_range,
            &mut account_res_z_range,
            &account.i_3_range,
            &account.x_3_range,
            current_index,
            FILLING_ROUNDS,
        )?; // 1 of 256
        account.current_index += 13;


        let mut account_g_ic_x_range = account.g_ic_x_range;
        let mut account_g_ic_y_range = account.g_ic_y_range;
        let mut account_g_ic_z_range = account.g_ic_z_range;
        maths_g_ic_instruction(
            &mut account_g_ic_x_range,
            &mut account_g_ic_y_range,
            &mut account_g_ic_z_range,
            &account_res_x_range,
            &account_res_y_range,
            &account_res_z_range,
        )?;
        account.g_ic_x_range = account_g_ic_x_range;
        account.g_ic_y_range = account_g_ic_y_range;
        account.g_ic_z_range = account_g_ic_z_range;

        init_res_instruction(
            &mut account_res_x_range,
            &mut account_res_y_range,
            &mut account_res_z_range,
        )?;
        account.res_x_range = account_res_x_range;
        account.res_y_range = account_res_y_range;
        account.res_z_range = account_res_z_range;


    } else if id == 45 {
        let mut account_res_x_range = account.res_x_range;
        let mut account_res_y_range = account.res_y_range;
        let mut account_res_z_range = account.res_z_range;
        maths_instruction(
            &mut account_res_x_range,
            &mut account_res_y_range,
            &mut account_res_z_range,
            &account.i_4_range,
            &account.x_4_range,
            current_index,
            ROUNDS,
        )?; // 1 of 256
        account.res_x_range = account_res_x_range;
        account.res_y_range = account_res_y_range;
        account.res_z_range = account_res_z_range;
        account.current_index += (ROUNDS as u64 / 4) - 1;


    } else if id == 65 {
        // last instruction after for 256 % ROUNDS != 0
        // executes FILLING_ROUNDS rounds
        let mut account_res_x_range = account.res_x_range;
        let mut account_res_y_range = account.res_y_range;
        let mut account_res_z_range = account.res_z_range;
        maths_instruction(
            &mut account_res_x_range,
            &mut account_res_y_range,
            &mut account_res_z_range,
            &account.i_4_range,
            &account.x_4_range,
            current_index,
            FILLING_ROUNDS,
        )?; // 1 of 256
        account.current_index += 13;

        let mut account_g_ic_x_range = account.g_ic_x_range;
        let mut account_g_ic_y_range = account.g_ic_y_range;
        let mut account_g_ic_z_range = account.g_ic_z_range;
        maths_g_ic_instruction(
            &mut account_g_ic_x_range,
            &mut account_g_ic_y_range,
            &mut account_g_ic_z_range,
            &account_res_x_range,
            &account_res_y_range,
            &account_res_z_range,
        )?;
        account.g_ic_x_range = account_g_ic_x_range;
        account.g_ic_y_range = account_g_ic_y_range;
        account.g_ic_z_range = account_g_ic_z_range;

        init_res_instruction(
            &mut account_res_x_range,
            &mut account_res_y_range,
            &mut account_res_z_range,
        )?;
        account.res_x_range = account_res_x_range;
        account.res_y_range = account_res_y_range;
        account.res_z_range = account_res_z_range;
    }
    // else if id == 46 {
    //     let mut account_g_ic_x_range = account.g_ic_x_range;
    //     let mut account_g_ic_y_range = account.g_ic_y_range;
    //     let mut account_g_ic_z_range = account.g_ic_z_range;
    //     maths_g_ic_instruction(
    //         &mut account_g_ic_x_range,
    //         &mut account_g_ic_y_range,
    //         &mut account_g_ic_z_range,
    //         &account.res_x_range,
    //         &account.res_y_range,
    //         &account.res_z_range,
    //     )?;
    //     account.g_ic_x_range = account_g_ic_x_range;
    //     account.g_ic_y_range = account_g_ic_y_range;
    //     account.g_ic_z_range = account_g_ic_z_range;
    //
    //
    // }
    else if id == 56 {
        let mut account_res_x_range = account.res_x_range;
        let mut account_res_y_range = account.res_y_range;
        let mut account_res_z_range = account.res_z_range;
        maths_instruction(
            &mut account_res_x_range,
            &mut account_res_y_range,
            &mut account_res_z_range,
            &account.i_5_range,
            &account.x_5_range,
            current_index,
            ROUNDS,
        )?; // 1 of 256
        account.res_x_range = account_res_x_range;
        account.res_y_range = account_res_y_range;
        account.res_z_range = account_res_z_range;
        account.current_index += (ROUNDS as u64 / 4) - 1;


    } else if id == 66 {
        // last instruction after for 256 % ROUNDS != 0
        // executes FILLING_ROUNDS rounds
        let mut account_res_x_range = account.res_x_range;
        let mut account_res_y_range = account.res_y_range;
        let mut account_res_z_range = account.res_z_range;
        maths_instruction(
            &mut account_res_x_range,
            &mut account_res_y_range,
            &mut account_res_z_range,
            &account.i_5_range,
            &account.x_5_range,
            current_index,
            FILLING_ROUNDS,
        )?; // 1 of 256
        account.current_index += 13;

        let mut account_g_ic_x_range = account.g_ic_x_range;
        let mut account_g_ic_y_range = account.g_ic_y_range;
        let mut account_g_ic_z_range = account.g_ic_z_range;
        maths_g_ic_instruction(
            &mut account_g_ic_x_range,
            &mut account_g_ic_y_range,
            &mut account_g_ic_z_range,
            &account_res_x_range,
            &account_res_y_range,
            &account_res_z_range,
        )?;
        account.g_ic_x_range = account_g_ic_x_range;
        account.g_ic_y_range = account_g_ic_y_range;
        account.g_ic_z_range = account_g_ic_z_range;

        init_res_instruction(
            &mut account_res_x_range,
            &mut account_res_y_range,
            &mut account_res_z_range,
        )?;
        account.res_x_range = account_res_x_range;
        account.res_y_range = account_res_y_range;
        account.res_z_range = account_res_z_range;

    }  else if id == 57 {
        let mut account_res_x_range = account.res_x_range;
        let mut account_res_y_range = account.res_y_range;
        let mut account_res_z_range = account.res_z_range;
        maths_instruction(
            &mut account_res_x_range,
            &mut account_res_y_range,
            &mut account_res_z_range,
            &account.i_6_range,
            &account.x_6_range,
            current_index,
            ROUNDS,
        )?; // 1 of 256
        account.res_x_range = account_res_x_range;
        account.res_y_range = account_res_y_range;
        account.res_z_range = account_res_z_range;
        account.current_index += (ROUNDS as u64 / 4) - 1;


    } else if id == 67 {
        // last instruction after for 256 % ROUNDS != 0
        // executes FILLING_ROUNDS rounds
        let mut account_res_x_range = account.res_x_range;
        let mut account_res_y_range = account.res_y_range;
        let mut account_res_z_range = account.res_z_range;
        maths_instruction(
            &mut account_res_x_range,
            &mut account_res_y_range,
            &mut account_res_z_range,
            &account.i_6_range,
            &account.x_6_range,
            current_index,
            FILLING_ROUNDS,
        )?; // 1 of 256
        account.current_index += 13;

        let mut account_g_ic_x_range = account.g_ic_x_range;
        let mut account_g_ic_y_range = account.g_ic_y_range;
        let mut account_g_ic_z_range = account.g_ic_z_range;
        maths_g_ic_instruction(
            &mut account_g_ic_x_range,
            &mut account_g_ic_y_range,
            &mut account_g_ic_z_range,
            &account_res_x_range,
            &account_res_y_range,
            &account_res_z_range,
        )?;
        account.g_ic_x_range = account_g_ic_x_range;
        account.g_ic_y_range = account_g_ic_y_range;
        account.g_ic_z_range = account_g_ic_z_range;

        init_res_instruction(
            &mut account_res_x_range,
            &mut account_res_y_range,
            &mut account_res_z_range,
        )?;
        account.res_x_range = account_res_x_range;
        account.res_y_range = account_res_y_range;
        account.res_z_range = account_res_z_range;

    } else if id == 58 {
        let mut account_res_x_range = account.res_x_range;
        let mut account_res_y_range = account.res_y_range;
        let mut account_res_z_range = account.res_z_range;
        maths_instruction(
            &mut account_res_x_range,
            &mut account_res_y_range,
            &mut account_res_z_range,
            &account.i_7_range,
            &account.x_7_range,
            current_index,
            ROUNDS,
        )?; // 1 of 256
        account.res_x_range = account_res_x_range;
        account.res_y_range = account_res_y_range;
        account.res_z_range = account_res_z_range;
        account.current_index += (ROUNDS as u64 / 4) - 1;

    } else if id == 68 {
        // last instruction after for 256 % ROUNDS != 0
        // executes FILLING_ROUNDS rounds
        let mut account_res_x_range = account.res_x_range;
        let mut account_res_y_range = account.res_y_range;
        let mut account_res_z_range = account.res_z_range;
        maths_instruction(
            &mut account_res_x_range,
            &mut account_res_y_range,
            &mut account_res_z_range,
            &account.i_7_range,
            &account.x_7_range,
            current_index,
            FILLING_ROUNDS,
        )?; // 1 of 256
        account.current_index += 13;

        account.res_x_range = account_res_x_range;
        account.res_y_range = account_res_y_range;
        account.res_z_range = account_res_z_range;

        let mut account_g_ic_x_range = account.g_ic_x_range;
        let mut account_g_ic_y_range = account.g_ic_y_range;
        let mut account_g_ic_z_range = account.g_ic_z_range;
        maths_g_ic_instruction(
            &mut account_g_ic_x_range,
            &mut account_g_ic_y_range,
            &mut account_g_ic_z_range,
            &account_res_x_range,
            &account_res_y_range,
            &account_res_z_range,
        )?;
        g_ic_into_affine_1(
            &mut account_g_ic_x_range,
            &mut account_g_ic_y_range,
            &mut account_g_ic_z_range, // only one changing
        )?;
        account.g_ic_x_range = account_g_ic_x_range;
        account.g_ic_y_range = account_g_ic_y_range;
        account.g_ic_z_range = account_g_ic_z_range;

        g_ic_into_affine_2(
            &account.g_ic_x_range.clone(),
            &account.g_ic_y_range.clone(),
            &account.g_ic_z_range.clone(),
            &mut account.x_1_range,
        )?;


    }
    // else if id == 47 {
    //     let mut account_g_ic_x_range = account.g_ic_x_range;
    //     let mut account_g_ic_y_range = account.g_ic_y_range;
    //     let mut account_g_ic_z_range = account.g_ic_z_range;
    //     g_ic_into_affine_1(
    //         &mut account_g_ic_x_range,
    //         &mut account_g_ic_y_range,
    //         &mut account_g_ic_z_range, // only one changing
    //     )?;
    //     account.g_ic_x_range = account_g_ic_x_range;
    //     account.g_ic_y_range = account_g_ic_y_range;
    //     account.g_ic_z_range = account_g_ic_z_range;
    //
    //
    // } else if id == 48 {
    //     g_ic_into_affine_2(
    //         &account.g_ic_x_range.clone(),
    //         &account.g_ic_y_range.clone(),
    //         &account.g_ic_z_range.clone(),
    //         &mut account.x_1_range,
    //     )?;
    //     let indices = [X_1_RANGE_INDEX];
    //
    // }
    Ok(())
}

// All 1809 instructions will be called in a fixed order. This should provide some safety.
// Also, only the first ix receives payload from the client (init_pairs_instruction).
// And since we don't read any payloads after the first ix, prepared_inputs can (theoretically) be executed within 2 blocks.

// How to read the ix_order in IX_ORDER_ARRAY:
// 40 - init_pairs; stores public inputs (i,x pairs) + initial g_ic in account once.
// As we'll see below, that's needed to replicate the loop behavior of the library implementation.
// (What's g_ic? In the end g_ic will hold the final value of prepared_inputs and be used by the verifier.)
// 41 - creates fresh res range. Res is like a temporary g_ic. This ix is called at the start of every round in the loop.
// The loop is essentially replicating the behavior of the lib implementation of prepare_inputs:
//  for (i, b) in public_inputs.iter().zip(pvk.vk.gamma_abc_g1.iter().skip(1)) {
//      g_ic.add_assign(&b.mul(i.into_repr()));
//  }
// The above for-loop is called 7 times because this implementation deals with 7 public inputs.
// Inside &b.mul(i) we have another loop that is always called 256 times:
//  let bits: ark_ff::BitIteratorBE<ark_ff::BigInteger256> = BitIteratorBE::new(a.into());
// That's why the next 256 ix_ids in the IX_ORDER_ARRAY are: 42.
// 42 - maths_instruction; does calculation akin to b.mul.
// After calling 42 ix for 256 times, we find the ix_id 46.
// 46 - maths_g_ic_instruction; updates g_ic with current res.
// This is needed since res is temporary and will be newly initialized at the start of the next loop.
// Looking at the IX_ORDER_ARRAY we can now see that the loop starts anew (41,43*256times,46,...).
// This continues for a total of 7 times because 7 inputs.
// Note: As we can see at every new round the 256 b.mul ix have different ix_ids (42,43,44,45,56,57,58).
// That's because we're accessing different i,x ranges. If you look
// at the actual calls inside /processor.rs you'll see the minor differences between those.

pub const IX_ORDER_ARRAY: [u8; 464] = [
    41, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
    42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
    42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 46, 41, 43, 43, 43, 43, 43,
    43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43,
    43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43,
    43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 46, 41, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44,
    44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44,
    44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44,
    44, 44, 44, 44, 44, 46, 41, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
    45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
    45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 46,
    41, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56,
    56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56,
    56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 46, 41, 57, 57, 57, 57, 57,
    57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57,
    57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57,
    57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 46, 41, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58,
    58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58,
    58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58,
    58, 58, 58, 58, 58, 46, 47, 48,
];

// The current_index informs the maths_instruction where exactly in the 256* loop we are at any given time.
// This is needed because we have to skip leading zeroes and can't keep
// track of its state. So we strip anew in every ix call:
//  let bits_without_leading_zeroes: Vec<bool> = bits.skip_while(|b| !b).collect();
//  let skipped = 256 - bits_without_leading_zeroes.len();
//  if current_index < skipped {
//      // "skipping leading zero instruction..."
//     return;
//  } else {
//      // "..."
//  }
// For every maths_instruction (one of 42,43,44,45,56,57,58) we count 0..256 -> current_instruction.
// Other instructions ignore current_index (see @processor) as they don't need it.

pub const CURRENT_INDEX_ARRAY: [u8; 464] = [
    41,
    0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44, 48, 52, 56, 60, 64, 68, 72, 76, 80, 84, 88,
    92, 96, 100, 104, 108, 112, 116, 120, 124, 128, 132, 136, 140, 144, 148, 152, 156, 160, 164,
    168, 172, 176, 180, 184, 188, 192, 196, 200, 204, 208, 212, 216, 220, 224, 228, 232, 236, 240,
    244, 248, 252, 46, 41,
    0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44, 48, 52, 56, 60, 64, 68, 72,
    76, 80, 84, 88, 92, 96, 100, 104, 108, 112, 116, 120, 124, 128, 132, 136, 140, 144, 148, 152,
    156, 160, 164, 168, 172, 176, 180, 184, 188, 192, 196, 200, 204, 208, 212, 216, 220, 224, 228,
    232, 236, 240, 244, 248, 252, 46, 41,
    0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44, 48, 52, 56,
    60, 64, 68, 72, 76, 80, 84, 88, 92, 96, 100, 104, 108, 112, 116, 120, 124, 128, 132, 136, 140,
    144, 148, 152, 156, 160, 164, 168, 172, 176, 180, 184, 188, 192, 196, 200, 204, 208, 212, 216,
    220, 224, 228, 232, 236, 240, 244, 248, 252, 46, 41, 0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40,
    44, 48, 52, 56, 60, 64, 68, 72, 76, 80, 84, 88, 92, 96, 100, 104, 108, 112, 116, 120, 124, 128,
    132, 136, 140, 144, 148, 152, 156, 160, 164, 168, 172, 176, 180, 184, 188, 192, 196, 200, 204,
    208, 212, 216, 220, 224, 228, 232, 236, 240, 244, 248, 252, 46, 41, 0, 4, 8, 12, 16, 20, 24,
    28, 32, 36, 40, 44, 48, 52, 56, 60, 64, 68, 72, 76, 80, 84, 88, 92, 96, 100, 104, 108, 112,
    116, 120, 124, 128, 132, 136, 140, 144, 148, 152, 156, 160, 164, 168, 172, 176, 180, 184, 188,
    192, 196, 200, 204, 208, 212, 216, 220, 224, 228, 232, 236, 240, 244, 248, 252, 46, 41, 0, 4,
    8, 12, 16, 20, 24, 28, 32, 36, 40, 44, 48, 52, 56, 60, 64, 68, 72, 76, 80, 84, 88, 92, 96, 100,
    104, 108, 112, 116, 120, 124, 128, 132, 136, 140, 144, 148, 152, 156, 160, 164, 168, 172, 176,
    180, 184, 188, 192, 196, 200, 204, 208, 212, 216, 220, 224, 228, 232, 236, 240, 244, 248, 252,
    46, 41, 0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44, 48, 52, 56, 60, 64, 68, 72, 76, 80, 84,
    88, 92, 96, 100, 104, 108, 112, 116, 120, 124, 128, 132, 136, 140, 144, 148, 152, 156, 160,
    164, 168, 172, 176, 180, 184, 188, 192, 196, 200, 204, 208, 212, 216, 220, 224, 228, 232, 236,
    240, 244, 248, 252, 46, 47, 48,
];
