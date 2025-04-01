pub use crate::Result;
use light_compressed_account::instruction_data::zero_copy::{
    ZInstructionDataInvokeCpi, ZPackedReadOnlyAddress, ZPackedReadOnlyCompressedAccount,
};
#[cfg(feature = "bench-sbf")]
use light_heap::{bench_sbf_end, bench_sbf_start};
use light_zero_copy::slice::ZeroCopySliceBorsh;

use super::verify_signer::cpi_signer_checks;
use crate::{
    account_traits::SignerAccounts, invoke_cpi::instruction::InvokeCpiInstruction,
    processor::process::process,
};

/// Processes an `InvokeCpi` instruction.
/// Checks:
/// 1. signer checks (inputs), write access (outputs) (cpi_signer_checks)
/// 2. sets or gets cpi context (process_cpi_context)
#[allow(unused_mut)]
pub fn process_invoke_cpi<'a, 'b, 'c: 'info + 'b, 'info>(
    mut ctx: Context<'a, 'b, 'c, 'info, InvokeCpiInstruction<'info>>,
    inputs: ZInstructionDataInvokeCpi<'a>,
    read_only_addresses: Option<ZeroCopySliceBorsh<'a, ZPackedReadOnlyAddress>>,
    read_only_accounts: Option<ZeroCopySliceBorsh<'a, ZPackedReadOnlyCompressedAccount>>,
) -> Result<()> {
    #[cfg(feature = "bench-sbf")]
    bench_sbf_start!("cpda_cpi_signer_checks");
    cpi_signer_checks(
        &ctx.invoking_program.key(),
        &ctx.get_authority().key(),
        &inputs.input_compressed_accounts_with_merkle_context,
        &inputs.output_compressed_accounts,
    )?;
    #[cfg(feature = "bench-sbf")]
    bench_sbf_end!("cpda_cpi_signer_checks");
    #[cfg(feature = "bench-sbf")]
    bench_sbf_start!("cpda_process_cpi_context");
    #[allow(unused)]
    let mut cpi_context_inputs_len = if let Some(value) = ctx.cpi_context_account.as_ref() {
        value.context.len()
    } else {
        0
    };
    let inputs = match crate::invoke_cpi::process_cpi_context::process_cpi_context(
        inputs,
        &mut ctx.cpi_context_account,
        ctx.fee_payer.key(),
        ctx.remaining_accounts,
    ) {
        Ok(Some(inputs)) => inputs,
        Ok(None) => return Ok(()),
        Err(err) => return Err(err),
    };
    #[cfg(feature = "bench-sbf")]
    bench_sbf_end!("cpda_process_cpi_context");

    process(
        inputs.into(),
        Some(ctx.invoking_program.key()),
        ctx,
        cpi_context_inputs_len,
        read_only_addresses,
        read_only_accounts,
    )
}
