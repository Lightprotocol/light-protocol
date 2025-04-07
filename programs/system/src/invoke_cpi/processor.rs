use light_compressed_account::instruction_data::traits::InstructionDataTrait;
#[cfg(feature = "bench-sbf")]
use light_heap::{bench_sbf_end, bench_sbf_start};
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};

pub use crate::Result;
use crate::{
    accounts::account_traits::{CpiContextAccountTrait, InvokeAccounts, SignerAccounts},
    context::WrappedInstructionData,
    errors::SystemProgramError,
    invoke_cpi::{account::deserialize_cpi_context_account, verify_signer::cpi_signer_checks},
    processor::process::process,
};

/// Processes an `InvokeCpi` instruction.
/// Checks:
/// 1. signer checks (inputs), write access (outputs) (cpi_signer_checks)
/// 2. sets or gets cpi context (process_cpi_context)
#[allow(unused_mut)]
pub fn process_invoke_cpi<
    'a,
    'info,
    T: InstructionDataTrait<'a>,
    A: SignerAccounts<'info> + InvokeAccounts<'info> + CpiContextAccountTrait<'info>,
>(
    invoking_program: Pubkey,
    ctx: A,
    inputs: WrappedInstructionData<'a, T>,
    remaining_accounts: &'info [AccountInfo],
) -> Result<()> {
    #[cfg(feature = "bench-sbf")]
    bench_sbf_start!("cpda_process_cpi_context");

    #[cfg(feature = "bench-sbf")]
    bench_sbf_start!("cpda_cpi_signer_checks");
    cpi_signer_checks::<T>(&invoking_program, &ctx.get_authority().key(), &inputs)?;
    #[cfg(feature = "bench-sbf")]
    bench_sbf_end!("cpda_cpi_signer_checks");

    #[allow(unused)]
    let (cpi_context_inputs_len, inputs) =
        match crate::invoke_cpi::process_cpi_context::process_cpi_context(
            inputs,
            ctx.get_cpi_context_account(),
            *ctx.get_fee_payer().key(),
            remaining_accounts,
        ) {
            Ok(Some(inputs)) => inputs,
            Ok(None) => return Ok(()),
            Err(err) => return Err(err),
        };
    #[cfg(feature = "bench-sbf")]
    bench_sbf_end!("cpda_process_cpi_context");

    process(
        inputs,
        Some(invoking_program),
        &ctx,
        cpi_context_inputs_len,
        remaining_accounts,
    )?;

    // clear cpi context account
    if cpi_context_inputs_len > 0 {
        let mut cpi_context_account =
            deserialize_cpi_context_account(&ctx.get_cpi_context_account().unwrap())?;

        if cpi_context_account.context.is_empty() {
            // msg!("cpi context account : {:?}", cpi_context_account);
            // msg!("fee payer : {:?}", fee_payer);
            return Err(SystemProgramError::CpiContextEmpty.into());
        }
        // else if *cpi_context_account.fee_payer != fee_payer.into()
        //     || cpi_context.first_set_context()
        // {
        //     msg!("cpi context account : {:?}", cpi_context_account);
        //     msg!("fee payer : {:?}", fee_payer);
        //     msg!("cpi context  : {:?}", cpi_context);
        //     return Err(SystemProgramError::CpiContextFeePayerMismatch.into());
        // }
        // Reset cpi context account
        cpi_context_account.context.clear();
        *cpi_context_account.fee_payer = Pubkey::default().into();
    }
    Ok(())
}
