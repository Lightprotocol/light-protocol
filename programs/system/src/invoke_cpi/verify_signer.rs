use light_compressed_account::instruction_data::traits::InstructionDataTrait;
#[cfg(feature = "bench-sbf")]
use light_heap::{bench_sbf_end, bench_sbf_start};
use pinocchio::{
    msg,
    program_error::ProgramError,
    pubkey::{create_program_address, try_find_program_address, Pubkey},
};

use crate::{
    constants::CPI_AUTHORITY_PDA_SEED, context::WrappedInstructionData, errors::SystemProgramError,
    Result,
};
/// Checks:
/// 1. Invoking program is signer (cpi_signer_check)
/// 2. Input compressed accounts with data are owned by the invoking program
///    (input_compressed_accounts_signer_check)
/// 3. Output compressed accounts with data are owned by the invoking program
///    (output_compressed_accounts_write_access_check)
pub fn cpi_signer_checks<'a, T: InstructionDataTrait<'a>>(
    invoking_program_id: &Pubkey,
    authority: &Pubkey,
    inputs: &WrappedInstructionData<'a, T>,
) -> Result<()> {
    #[cfg(feature = "bench-sbf")]
    bench_sbf_start!("cpda_cpi_signer_checks");
    cpi_signer_check(invoking_program_id, authority, inputs.bump())?;
    #[cfg(feature = "bench-sbf")]
    bench_sbf_end!("cpda_cpi_signer_checks");
    #[cfg(feature = "bench-sbf")]
    bench_sbf_start!("cpd_input_checks");
    input_compressed_accounts_signer_check(inputs, invoking_program_id)?;
    #[cfg(feature = "bench-sbf")]
    bench_sbf_end!("cpd_input_checks");
    #[cfg(feature = "bench-sbf")]
    bench_sbf_start!("cpda_cpi_write_checks");
    output_compressed_accounts_write_access_check(inputs, invoking_program_id)?;
    #[cfg(feature = "bench-sbf")]
    bench_sbf_end!("cpda_cpi_write_checks");
    Ok(())
}

/// Cpi signer check, validates that the provided invoking program
/// is the actual invoking program.
pub fn cpi_signer_check(
    invoking_program: &Pubkey,
    authority: &Pubkey,
    bump: Option<u8>,
) -> Result<()> {
    let derived_signer = if let Some(bump) = bump {
        let seeds = [CPI_AUTHORITY_PDA_SEED, &[bump][..]];
        create_program_address(&seeds, invoking_program)?
    } else {
        let seeds = [CPI_AUTHORITY_PDA_SEED];
        try_find_program_address(&seeds, invoking_program)
            .ok_or(ProgramError::InvalidSeeds)?
            .0
    };
    if derived_signer != *authority {
        msg!(format!(
            "Cpi signer check failed. Derived cpi signer {:?} !=  authority {:?}",
            derived_signer, authority
        )
        .as_str());
        return Err(SystemProgramError::CpiSignerCheckFailed.into());
    }
    Ok(())
}

/// Checks that the invoking program owns all input compressed accounts.
pub fn input_compressed_accounts_signer_check<'a, 'info, T: InstructionDataTrait<'a>>(
    inputs: &WrappedInstructionData<'a, T>,
    invoking_program_id: &Pubkey,
) -> Result<()> {
    inputs.input_accounts()
        .try_for_each(
            |compressed_account_with_context| {
                if *invoking_program_id == compressed_account_with_context.owner().to_bytes() {
                    Ok(())
                } else {
                    msg!(
                        "Input signer check failed. Program cannot invalidate an account it doesn't own. Owner {:?} !=  invoking_program_id {:?}",
                        compressed_account_with_context.owner().to_bytes(),
                        invoking_program_id
                    );
                    Err(SystemProgramError::SignerCheckFailed.into())
                }
            },
        )
}

/// Write access check for output compressed accounts.
/// - Only program-owned output accounts can hold data.
/// - Every output account that holds data has to be owned by the
///     invoking_program.
/// - outputs without data can be owned by any pubkey.
#[inline(never)]
pub fn output_compressed_accounts_write_access_check<'a, 'info, T: InstructionDataTrait<'a>>(
    inputs: &WrappedInstructionData<'a, T>,
    invoking_program_id: &Pubkey,
) -> Result<()> {
    for compressed_account in inputs.output_accounts() {
        if compressed_account.has_data()
            && *invoking_program_id != compressed_account.owner().to_bytes()
        {
            msg!(
                 format!(   "Signer/Program cannot write into an account it doesn't own. Write access check failed compressed account owner {:?} !=  invoking_program_id {:?}",
                    compressed_account.owner().to_bytes(),
                    invoking_program_id
                ).as_str());
            // msg!(format!("compressed_account: {:?}", compressed_account).as_str());
            return Err(SystemProgramError::WriteAccessCheckFailed.into());
        }
        if !compressed_account.has_data()
            && *invoking_program_id == compressed_account.owner().to_bytes()
        {
            msg!("For program owned compressed accounts the data field needs to be defined.");
            // msg!("compressed_account: {:?}", compressed_account);
            return Err(SystemProgramError::DataFieldUndefined.into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_cpi_signer_check() {
        for _ in 0..1000 {
            let seeds = [CPI_AUTHORITY_PDA_SEED];
            let invoking_program = Pubkey::new_unique();
            let (derived_signer, _) = Pubkey::find_program_address(&seeds[..], &invoking_program);
            assert_eq!(cpi_signer_check(&invoking_program, &derived_signer), Ok(()));

            let authority = Pubkey::new_unique();
            let invoking_program = Pubkey::new_unique();
            assert!(
                cpi_signer_check(&invoking_program, &authority)
                    == Err(ProgramError::InvalidSeeds.into())
                    || cpi_signer_check(&invoking_program, &authority)
                        == Err(SystemProgramError::CpiSignerCheckFailed.into())
            );
        }
    }
}
