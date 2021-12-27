use ark_ff;
use ark_ff::bytes::FromBytes;
use ark_ff::Fp256;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    msg,
    program_error::ProgramError,
    program_pack::Pack,
};

use crate::pi_processor::_pi_254_process_instruction;
use crate::pi_state::PiBytes;
use crate::IX_ORDER;
/*
pub fn _pre_process_instruction(
    _instruction_data: &[u8],
    accounts: &[AccountInfo],
) -> Result<(), ProgramError> {
    let account = &mut accounts.iter();
    let _signing_account = next_account_info(account)?;

    let pi_account = next_account_info(account)?;


    let mut account_data = PiBytes::unpack(&pi_account.data.borrow())?;
    //remove 40 from instruction array then remove this
    if account_data.current_instruction_index == 0 {
        account_data.current_instruction_index += 1;
        PiBytes::pack_into_slice(&account_data, &mut pi_account.data.borrow_mut());
        return Ok(());
    }
    let mut inputs: Vec<Fp256<ark_bn254::FrParameters>> = vec![];
    // msg!(
    //     "Executing instruction: {}",
    //     IX_ORDER[account_data.current_instruction_index]
    // );

    let current_instruction_index = account_data.current_instruction_index;
    _pi_254_process_instruction(
        IX_ORDER[current_instruction_index],
        &mut account_data,
        &inputs,
        usize::from(CURRENT_INDEX_ARRAY[current_instruction_index - 1]),
    );

    account_data.current_instruction_index += 1;
    // msg!(
    //     "current_instruction_index: {}",
    //     account_data.current_instruction_index
    // );
    PiBytes::pack_into_slice(&account_data, &mut pi_account.data.borrow_mut());
    //}
    Ok(())
}
*/
// All 1809 instructions will be called in a fixed order. This should provide some safety.
// Also, only the first ix receives data from the client (init_pairs_instruction).
// And since we don't read ix_id from data fill_p/prepared_inputs can be executed within 2 blocks.
// TODO: Could we pass in the input data into each instruction? If yes, we'd only need 1 block.
// Though that'd be expensive, especially once Solana adds dynamic fees.
// Maybe we'd call the first ix in the same block as the ix for passing in the proof data @verifier.
// Then we'd have: 1(data) + 1(pi) + 1(verify) = 3 blocks for the whole withdrawal.
// That's about 1.5 sec. / 0.5 secs when Solana reaches 150ms blocks.
// We could probably do 2 blocks too if we execute in the same program?

// How to read the ix_order in IX_ORDER_ARRAY:
// 40 - init_pairs; stores public inputs (i,x pairs) + initial g_ic in account once.
// As we'll see below, that's needed to replicate the loop behavior of the library implementation.
// (What's g_ic? In the end g_ic will hold the final value of prepared_inputs and be used by the verifier.)
// 41 - creates fresh res range. Res is like a temp g_ic. This ix is called at the start of every round in the loop.
// The loop is essentially replicating the behavior of the lib implementation of prepare_inputs:
//  for (i, b) in public_inputs.iter().zip(pvk.vk.gamma_abc_g1.iter().skip(1)) {
//      g_ic.add_assign(&b.mul(i.into_repr()));
//  }
// The above for-loop is called 7 times because this implementation deals with 7 public inputs.
// Inside &b.mul(i) we have another loop that is always called 256 times:
//  let bits: ark_ff::BitIteratorBE<ark_ff::BigInteger256> = BitIteratorBE::new(a.into());
// That's why the next 256 ix_ids in the IX_ORDER_ARRAY are: 42.
// 42 - maths_instruction; does calculations akin to b.mul.
// After calling 42 ix for 256 times, we find the ix_id 46.
// 46 - maths_g_ic_instruction; updates g_ic with current res.
// This is needed since res is temp and will be newly initialized at the start of the next loop.
// Looking at the IX_ORDER_ARRAY we can now see that the loop stats anew (41,43*256times,46,...).
// This continues for a total of 7 times because 7 inputs.
// Note: As we can see at every new round the 256 b.mul ix have different ix_ids (42,43,44,45,56,57,58).
// That's because we're accessing different i,x ranges. If you look
// at the actual calls inside /pi_processor.rs you'll see the minor differences.


pub const IX_ORDER_ARRAY: [u8; 465] = [
    40, 41, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
    42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
    42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 46, 41, 43, 43, 43, 43,
    43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43,
    43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43,
    43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 46, 41, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44,
    44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44,
    44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44,
    44, 44, 44, 44, 44, 44, 46, 41, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
    45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
    45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
    46, 41, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56,
    56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56,
    56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 56, 46, 41, 57, 57, 57, 57,
    57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57,
    57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57,
    57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 57, 46, 41, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58,
    58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58,
    58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58, 58,
    58, 58, 58, 58, 58, 58, 46, 47, 48,
];

// The current_index informs the maths_instruction where we are in the 256* loop.
// This is needed because we have to skip leading zeroes and can't keep
// track of state. So we strip anew in every ix call:
//  let bits_without_leading_zeroes: Vec<bool> = bits.skip_while(|b| !b).collect();
//  let skipped = 256 - bits_without_leading_zeroes.len();
//  if current_index < skipped {
//      // "skipping leading zero instruction..."
//     return;
//  } else {
//      // "..."
//  }
// For every maths_instruction (one of 42,43,44,45,56,57,58) we count 0..256.
// Other instructions ignore current_index (see @processor).

pub const CURRENT_INDEX_ARRAY: [u8; 465] = [
    40, 41, 0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44, 48, 52, 56, 60, 64, 68, 72, 76, 80, 84,
    88, 92, 96, 100, 104, 108, 112, 116, 120, 124, 128, 132, 136, 140, 144, 148, 152, 156, 160,
    164, 168, 172, 176, 180, 184, 188, 192, 196, 200, 204, 208, 212, 216, 220, 224, 228, 232, 236,
    240, 244, 248, 252, 46, 41, 0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44, 48, 52, 56, 60, 64,
    68, 72, 76, 80, 84, 88, 92, 96, 100, 104, 108, 112, 116, 120, 124, 128, 132, 136, 140, 144,
    148, 152, 156, 160, 164, 168, 172, 176, 180, 184, 188, 192, 196, 200, 204, 208, 212, 216, 220,
    224, 228, 232, 236, 240, 244, 248, 252, 46, 41, 0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44,
    48, 52, 56, 60, 64, 68, 72, 76, 80, 84, 88, 92, 96, 100, 104, 108, 112, 116, 120, 124, 128,
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
    240, 244, 248, 252, 46, 41, 0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44, 48, 52, 56, 60, 64,
    68, 72, 76, 80, 84, 88, 92, 96, 100, 104, 108, 112, 116, 120, 124, 128, 132, 136, 140, 144,
    148, 152, 156, 160, 164, 168, 172, 176, 180, 184, 188, 192, 196, 200, 204, 208, 212, 216, 220,
    224, 228, 232, 236, 240, 244, 248, 252, 46, 47, 48,
];
