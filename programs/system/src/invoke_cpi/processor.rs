pub use crate::Result;
use light_compressed_account::instruction_data::zero_copy::{
    ZInstructionDataInvoke, ZInstructionDataInvokeCpi, ZPackedReadOnlyAddress,
    ZPackedReadOnlyCompressedAccount,
};
#[cfg(feature = "bench-sbf")]
use light_heap::{bench_sbf_end, bench_sbf_start};
use light_zero_copy::slice::ZeroCopySliceBorsh;

use super::verify_signer::cpi_signer_checks;
use crate::{
    account_traits::SignerAccounts,
    errors::SystemProgramError,
    invoke_cpi::{account::deserialize_cpi_context_account, instruction::InvokeCpiInstruction},
    processor::process::process,
};
use pinocchio::account_info::AccountInfo;

/// Processes an `InvokeCpi` instruction.
/// Checks:
/// 1. signer checks (inputs), write access (outputs) (cpi_signer_checks)
/// 2. sets or gets cpi context (process_cpi_context)
#[allow(unused_mut)]
pub fn process_invoke_cpi<'a, 'b, 'c: 'info + 'b, 'info>(
    mut ctx: InvokeCpiInstruction<'info>,
    inputs: ZInstructionDataInvokeCpi<'a>,
    read_only_addresses: Option<ZeroCopySliceBorsh<'a, ZPackedReadOnlyAddress>>,
    read_only_accounts: Option<ZeroCopySliceBorsh<'a, ZPackedReadOnlyCompressedAccount>>,
    remaining_accounts: &'b [AccountInfo],
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
    let (cpi_context_account, cpi_context_inputs_len) =
        if let Some(cpi_context) = inputs.cpi_context.as_ref() {
            if cpi_context.first_set_context() || cpi_context.set_context() {
                match crate::invoke_cpi::process_cpi_context::process_cpi_context(
                    inputs,
                    &mut ctx.cpi_context_account,
                    *ctx.fee_payer.key(),
                    remaining_accounts,
                ) {
                    Ok(Some(inputs)) => inputs,
                    Ok(None) => return Ok(()),
                    Err(err) => return Err(err),
                };
                unimplemented!()
            } else {
                let cpi_context_account =
                    deserialize_cpi_context_account(ctx.cpi_context_account.unwrap())?;

                if cpi_context_account.context.is_empty() {
                    // msg!("cpi context account : {:?}", cpi_context_account);
                    // msg!("fee payer : {:?}", fee_payer);
                    // msg!("cpi context  : {:?}", cpi_context);
                    return Err(SystemProgramError::CpiContextEmpty.into());
                } else if *cpi_context_account.fee_payer != ctx.fee_payer.key().into()
                    || cpi_context.first_set_context()
                {
                    // msg!("cpi context account : {:?}", cpi_context_account);
                    // msg!("fee payer : {:?}", fee_payer);
                    // msg!("cpi context  : {:?}", cpi_context);
                    return Err(SystemProgramError::CpiContextFeePayerMismatch.into());
                }

                // num_cpi_contexts = cpi_context_account.context.len();
                // Reset cpi context account
                let len = cpi_context_account.context.len();
                (Some(cpi_context_account), len)
            }
        } else {
            (None, 0)
        };
    // #[allow(unused)]
    // let (inputs, cpi_context_inputs_len) =
    //     match crate::invoke_cpi::process_cpi_context::process_cpi_context(
    //         inputs,
    //         &mut ctx.cpi_context_account,
    //         *ctx.fee_payer.key(),
    //         remaining_accounts,
    //     ) {
    //         Ok(Some(inputs)) => inputs,
    //         Ok(None) => return Ok(()),
    //         Err(err) => return Err(err),
    //     };
    #[cfg(feature = "bench-sbf")]
    bench_sbf_end!("cpda_process_cpi_context");
    let inputs: ZInstructionDataInvoke = inputs.into();
    let wrapped_inputs = crate::context::WrappedInstructionData::new(inputs, cpi_context_account);
    process(
        wrapped_inputs,
        Some(*ctx.invoking_program.key()),
        &ctx,
        cpi_context_inputs_len,
        read_only_addresses,
        read_only_accounts,
        remaining_accounts,
    )?;

    // Reset cpi context account.
    if cpi_context_inputs_len > 0 {
        let mut cpi_context_account =
            deserialize_cpi_context_account(ctx.cpi_context_account.unwrap())?;
        cpi_context_account.context = Vec::new();
        *cpi_context_account.fee_payer = light_compressed_account::pubkey::Pubkey::default();
    }

    Ok(())
}
