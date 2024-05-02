use super::{process_cpi_context::process_cpi_context, verify_signer::cpi_signer_checks};
use crate::{
    invoke::processor::process, invoke_cpi::instruction::InvokeCpiInstruction,
    sdk::accounts::SignerAccounts, InstructionDataInvoke, InstructionDataInvokeCpi,
};
pub use anchor_lang::prelude::*;

pub fn process_invoke_cpi<'a, 'b, 'c: 'info + 'b, 'info>(
    mut ctx: Context<'a, 'b, 'c, 'info, InvokeCpiInstruction<'info>>,
    inputs: InstructionDataInvokeCpi,
) -> Result<()> {
    cpi_signer_checks(
        &inputs.signer_seeds,
        &ctx.accounts.invoking_program.key(),
        &ctx.accounts.get_authority().key(),
        &inputs,
    )?;

    let inputs = match process_cpi_context(inputs, &mut ctx) {
        Ok(Some(inputs)) => inputs,
        Ok(None) => return Ok(()),
        Err(err) => return Err(err),
    };

    process(
        InstructionDataInvoke {
            input_compressed_accounts_with_merkle_context: inputs
                .input_compressed_accounts_with_merkle_context,
            output_compressed_accounts: inputs.output_compressed_accounts,
            relay_fee: inputs.relay_fee,
            input_root_indices: inputs.input_root_indices,
            output_state_merkle_tree_account_indices: inputs
                .output_state_merkle_tree_account_indices,
            proof: inputs.proof,
            new_address_params: inputs.new_address_params,
            compression_lamports: inputs.compression_lamports,
            is_compress: inputs.is_compress,
        },
        Some(ctx.accounts.invoking_program.key()),
        ctx,
    )
}
