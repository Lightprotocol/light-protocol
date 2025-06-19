use light_account_checks::AccountInfoTrait;
use light_compressed_account::instruction_data::traits::InstructionData;
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};

pub use crate::Result;
use crate::{
    accounts::account_traits::{CpiContextAccountTrait, InvokeAccounts, SignerAccounts},
    context::WrappedInstructionData,
    invoke_cpi::{process_cpi_context::process_cpi_context, verify_signer::cpi_signer_checks},
    processor::process::process,
};

/// Processes an `InvokeCpi` instruction.
/// Checks:
/// 1. signer checks (instruction_data), write access (outputs) (cpi_signer_checks)
/// 2. sets or gets cpi context (process_cpi_context)
/// 3. Process input data and cpi account compression program.
/// 4. Clears the cpi context account if used.
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
    pinocchio::msg!("post cpi_signer_checks");
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
    pinocchio::msg!("post process_cpi_context");
    let remaining_accounts_pubkeys = remaining_accounts
        .iter()
        .map(|x| solana_pubkey::Pubkey::new_from_array(x.pubkey()))
        .collect::<Vec<_>>();
    pinocchio::msg!(format!("remaining_accounts {:?}", remaining_accounts_pubkeys).as_str());
    instruction_data
        .new_addresses()
        .for_each(|x| pinocchio::msg!(format!("instruction_data {:?}", x).as_str()));

    // 3. Process input data and cpi the account compression program.
    process::<ADDRESS_ASSIGNMENT, A, T>(
        instruction_data,
        Some(invoking_program),
        &accounts,
        cpi_context_inputs_len,
        remaining_accounts,
        accounts.get_cpi_context_account(),
    )?;

    // 4. clear cpi context account
    if cpi_context_inputs_len > 0 {
        clear_cpi_context_account(accounts.get_cpi_context_account())?;
    }
    Ok(())
}

/// Clear the CPI context account by setting the length to 0.
pub fn clear_cpi_context_account(account_info: Option<&AccountInfo>) -> Result<()> {
    let mut data = account_info.unwrap().try_borrow_mut_data()?;
    let start_offset = 8 + 32 + 32;
    data[start_offset..start_offset + 4].copy_from_slice(&[0u8, 0u8, 0u8, 0u8]);
    Ok(())
}
