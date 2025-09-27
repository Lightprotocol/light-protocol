use light_compressed_account::instruction_data::traits::InstructionData;
use light_program_profiler::profile;
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};

pub use crate::Result;
use crate::{
    accounts::account_traits::{CpiContextAccountTrait, InvokeAccounts, SignerAccounts},
    context::WrappedInstructionData,
    cpi_context::{
        process_cpi_context::process_cpi_context, state::deserialize_cpi_context_account_cleared,
    },
    invoke_cpi::verify_signer::cpi_signer_checks,
    processor::process::process,
};

/// Processes an `InvokeCpi` instruction.
/// Checks:
/// 1. signer checks (instruction_data), write access (outputs) (cpi_signer_checks)
/// 2. sets or gets cpi context (process_cpi_context)
/// 3. Process input data and cpi account compression program.
/// 4. Clears the cpi context account if used.
#[profile]
#[allow(unused_mut)]
pub fn process_invoke_cpi<
    'a,
    'info,
    const ADDRESS_ASSIGNMENT: bool,
    A: SignerAccounts<'info> + InvokeAccounts<'info> + CpiContextAccountTrait<'info>,
    T: InstructionData<'a>,
>(
    invoking_program: Pubkey,
    accounts: A,
    instruction_data: T,
    remaining_accounts: &'info [AccountInfo],
) -> Result<()> {
    let instruction_data = WrappedInstructionData::new(instruction_data)?;

    cpi_signer_checks::<T>(
        &invoking_program,
        accounts.get_authority().key(),
        &instruction_data,
    )?;

    let (cpi_context_inputs_len, instruction_data) = match process_cpi_context(
        instruction_data,
        accounts.get_cpi_context_account(),
        *accounts.get_fee_payer().key(),
        remaining_accounts,
    ) {
        Ok(Some(instruction_data)) => instruction_data,
        Ok(None) => return Ok(()),
        Err(err) => return Err(err),
    };
    // 3. Process input data and cpi the account compression program.
    process::<ADDRESS_ASSIGNMENT, A, T>(
        instruction_data,
        Some(invoking_program),
        &accounts,
        cpi_context_inputs_len,
        remaining_accounts,
    )?;

    // 4. clear cpi context account
    if cpi_context_inputs_len > 0 {
        deserialize_cpi_context_account_cleared(accounts.get_cpi_context_account().unwrap())?;
    }
    Ok(())
}
