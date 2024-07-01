pub use anchor_lang::prelude::*;
use light_heap::{bench_sbf_end, bench_sbf_start};

use super::verify_signer::cpi_signer_checks;
use crate::{
    invoke::processor::process, invoke_cpi::instruction::InvokeCpiInstruction,
    sdk::accounts::SignerAccounts, InstructionDataInvoke, InstructionDataInvokeCpi,
};

/// Processes an `InvokeCpi` instruction.
/// Checks:
/// 1. signer checks (inputs), writeaccess (outputs) (cpi_signer_checks)
/// 2. sets or gets cpi context (process_cpi_context)
#[allow(unused_mut)]
pub fn process_invoke_cpi<'a, 'b, 'c: 'info + 'b, 'info>(
    mut ctx: Context<'a, 'b, 'c, 'info, InvokeCpiInstruction<'info>>,
    inputs: InstructionDataInvokeCpi,
) -> Result<()> {
    bench_sbf_start!("cpda_cpi_signer_checks");
    cpi_signer_checks(
        &ctx.accounts.invoking_program.key(),
        &ctx.accounts.get_authority().key(),
        &inputs.signer_seeds,
        &inputs.input_compressed_accounts_with_merkle_context,
        &inputs.output_compressed_accounts,
    )?;
    bench_sbf_end!("cpda_cpi_signer_checks");
    bench_sbf_start!("cpda_process_cpi_context");
    #[allow(unused)]
    let mut cpi_context_inputs_len = if let Some(value) = ctx.accounts.cpi_context_account.as_ref()
    {
        value.context.len()
    } else {
        0
    };
    let inputs = match crate::invoke_cpi::process_cpi_context::process_cpi_context(
        inputs,
        &mut ctx.accounts.cpi_context_account,
        ctx.accounts.fee_payer.key(),
        ctx.remaining_accounts,
    ) {
        Ok(Some(inputs)) => inputs,
        Ok(None) => return Ok(()),
        Err(err) => return Err(err),
    };
    bench_sbf_end!("cpda_process_cpi_context");

    let data = InstructionDataInvoke {
        input_compressed_accounts_with_merkle_context: inputs
            .input_compressed_accounts_with_merkle_context,
        output_compressed_accounts: inputs.output_compressed_accounts,
        relay_fee: inputs.relay_fee,
        proof: inputs.proof,
        new_address_params: inputs.new_address_params,
        compress_or_decompress_lamports: inputs.compress_or_decompress_lamports,
        is_compress: inputs.is_compress,
    };
    process(
        data,
        Some(ctx.accounts.invoking_program.key()),
        ctx,
        cpi_context_inputs_len,
    )
}
