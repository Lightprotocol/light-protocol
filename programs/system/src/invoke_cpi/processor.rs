use light_compressed_account::instruction_data::traits::InstructionData;
#[cfg(feature = "bench-sbf")]
use light_heap::{bench_sbf_end, bench_sbf_start};
use pinocchio::{account_info::AccountInfo, msg, pubkey::Pubkey};

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
    const ADDRESS_ASSIGNMENT: bool,
    A: SignerAccounts<'info> + InvokeAccounts<'info> + CpiContextAccountTrait<'info>,
    T: InstructionData<'a>,
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
    cpi_signer_checks::<T>(&invoking_program, ctx.get_authority().key(), &inputs)?;
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

    process::<ADDRESS_ASSIGNMENT, A, T>(
        inputs,
        Some(invoking_program),
        &ctx,
        cpi_context_inputs_len,
        remaining_accounts,
    )?;

    // clear cpi context account
    if cpi_context_inputs_len > 0 {
        msg!("cpi_context_inputs_len");
        let mut cpi_context_account =
            deserialize_cpi_context_account(ctx.get_cpi_context_account().unwrap())?;
        msg!("cpi_context_inputs_len1");

        if cpi_context_account.context.is_empty() {
            msg!(format!("cpi context account : {:?}", cpi_context_account).as_str());
            return Err(SystemProgramError::CpiContextEmpty.into());
        }
        msg!("cpi_context_inputs_len2");
        // Reset cpi context account
        cpi_context_account.context.clear();
        msg!("cpi_context_inputs_len3");
        *cpi_context_account.fee_payer = Pubkey::default().into();
        msg!("cpi_context_inputs_len4");
    }
    Ok(())
}
