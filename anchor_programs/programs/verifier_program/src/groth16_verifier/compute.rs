use anchor_lang::prelude::*;

use crate::groth16_verifier::{
    prepare_inputs::*,
    final_exponentiation_process_instruction,
    miller_loop::*,
    parsers::*,
    VerifierState,
};
use merkle_tree_program::utils::constants::STORAGE_SEED;

use crate::errors::ErrorCode;

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
    let tmp_account = &mut ctx.accounts.verifier_state.load_mut()?;

    if tmp_account.computing_prepared_inputs
    {
        msg!(
            "CURRENT_INDEX_ARRAY[tmp_account.current_index as usize]: {}",
            CURRENT_INDEX_ARRAY[tmp_account.current_index as usize]
        );
        _process_instruction(
            IX_ORDER[tmp_account.current_instruction_index as usize],
            tmp_account,
            usize::from(CURRENT_INDEX_ARRAY[tmp_account.current_index as usize]),
        )?;
        tmp_account.current_index += 1;
    } else if tmp_account.computing_miller_loop {
        tmp_account.ml_max_compute = 1_300_000;

        msg!(
            "computing miller_loop {}",
            tmp_account.current_instruction_index
        );
        miller_loop_process_instruction(tmp_account);
    } else {
        if !tmp_account.computing_final_exponentiation {
            msg!("Initializing for final_exponentiation.");
            tmp_account.computing_final_exponentiation = true;
            let mut f1 = parse_f_from_bytes(&tmp_account.f_bytes.to_vec());
            f1.conjugate();
            tmp_account.f_bytes1 = parse_f_to_bytes(f1);
            // Initializing temporary storage for final_exponentiation
            // with fqk::zero() which is equivalent to [[1], [0;383]].concat()
            tmp_account.f_bytes2[0] = 1;
            tmp_account.f_bytes3[0] = 1;
            tmp_account.f_bytes4[0] = 1;
            tmp_account.f_bytes5[0] = 1;
            tmp_account.i_bytes[0] = 1;
            // Skipping the first loop iteration since the naf_vec is zero.
            tmp_account.outer_loop = 1;
            // Adjusting max compute limite to 1.2m, we still need some buffer
            // for overhead and varying compute costs depending on the numbers.
            tmp_account.fe_max_compute = 1_200_000;
            // Adding compute costs for packing the initialized fs.
            tmp_account.current_compute+=150_000;
        }

        msg!("Computing final_exponentiation");
        final_exponentiation_process_instruction(tmp_account);
    }

    tmp_account.current_instruction_index += 1;
    Ok(())
}
