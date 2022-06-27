use anchor_lang::prelude::*;

use crate::groth16_verifier::{
    prepare_inputs::*,
    final_exponentiation_process_instruction,
    miller_loop::*,
    parsers::*,
    VerifierState,
};
use merkle_tree_program::utils::constants::STORAGE_SEED;

#[derive(Accounts)]
pub struct Compute<'info> {
    #[account(
        mut,
        seeds = [verifier_state.load()?.tx_integrity_hash.as_ref(), STORAGE_SEED.as_ref()],
        bump
    )]
    pub verifier_state: AccountLoader<'info, VerifierState>,
    #[account(mut, address=verifier_state.load()?.signing_address)]
    pub signing_address: Signer<'info>
}

pub fn process_compute(ctx: Context<Compute>) -> Result<()> {
    let verifier_state_data = &mut ctx.accounts.verifier_state.load_mut()?;

    if verifier_state_data.computing_prepared_inputs
    {
        msg!(
            "CURRENT_INDEX_ARRAY[verifier_state_data.current_index as usize]: {}",
            CURRENT_INDEX_ARRAY[verifier_state_data.current_index as usize]
        );
        _process_instruction(
            IX_ORDER[verifier_state_data.current_instruction_index as usize],
            verifier_state_data,
            usize::from(CURRENT_INDEX_ARRAY[verifier_state_data.current_index as usize]),
        )?;
        verifier_state_data.current_index += 1;
    } else if verifier_state_data.computing_miller_loop {
        verifier_state_data.ml_max_compute = 1_300_000;

        msg!(
            "computing miller_loop {}",
            verifier_state_data.current_instruction_index
        );
        miller_loop_process_instruction(verifier_state_data);
    } else {
        if !verifier_state_data.computing_final_exponentiation {
            msg!("Initializing for final_exponentiation.");
            verifier_state_data.computing_final_exponentiation = true;
            let mut f1 = parse_f_from_bytes(&verifier_state_data.f_bytes.to_vec());
            f1.conjugate();
            verifier_state_data.f_bytes1 = parse_f_to_bytes(f1);
            // Initializing temporary storage for final_exponentiation
            // with fqk::zero() which is equivalent to [[1], [0;383]].concat()
            verifier_state_data.f_bytes2[0] = 1;
            verifier_state_data.f_bytes3[0] = 1;
            verifier_state_data.f_bytes4[0] = 1;
            verifier_state_data.f_bytes5[0] = 1;
            verifier_state_data.i_bytes[0] = 1;
            // Skipping the first loop iteration since the naf_vec is zero.
            verifier_state_data.outer_loop = 1;
            // Adjusting max compute limite to 1.2m, we still need some buffer
            // for overhead and varying compute costs depending on the numbers.
            verifier_state_data.fe_max_compute = 1_200_000;
            // Adding compute costs for packing the initialized fs.
            verifier_state_data.current_compute+=150_000;
        }

        msg!("Computing final_exponentiation");
        final_exponentiation_process_instruction(verifier_state_data);
    }

    verifier_state_data.current_instruction_index += 1;
    Ok(())
}
