#![allow(unused_imports)]
use light_compressed_account::instruction_data::traits::InstructionData;
use light_program_profiler::profile;
use pinocchio::{
    msg,
    program_error::ProgramError,
    pubkey::{checked_create_program_address, try_find_program_address, Pubkey},
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
#[profile]
pub fn cpi_signer_checks<'a, T: InstructionData<'a>>(
    invoking_program_id: &Pubkey,
    authority: &Pubkey,
    inputs: &WrappedInstructionData<'a, T>,
) -> Result<()> {
    cpi_signer_check(invoking_program_id, authority, inputs.bump())?;

    input_compressed_accounts_signer_check(inputs, invoking_program_id)?;

    output_compressed_accounts_write_access_check(inputs, invoking_program_id)?;
    Ok(())
}

#[allow(unused_variables)]
/// Cpi signer check, validates that the provided invoking program
/// is the actual invoking program.
#[profile]
pub fn cpi_signer_check(
    invoking_program: &Pubkey,
    authority: &Pubkey,
    bump: Option<u8>,
) -> Result<()> {
    let derived_signer = if let Some(bump) = bump {
        let seeds = [CPI_AUTHORITY_PDA_SEED];
        pinocchio_pubkey::derive_address(&seeds, Some(bump), invoking_program)
    } else {
        // Kept for backwards compatibility with instructions, invoke, and invoke cpi.
        let seeds = [CPI_AUTHORITY_PDA_SEED];
        solana_pubkey::Pubkey::try_find_program_address(
            &seeds,
            &solana_pubkey::Pubkey::new_from_array(*invoking_program),
        )
        .ok_or(ProgramError::InvalidSeeds)?
        .0
        .to_bytes()
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
#[profile]
pub fn input_compressed_accounts_signer_check<'a, 'info, T: InstructionData<'a>>(
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
                       format!("Input signer check failed. Program cannot invalidate an account it doesn't own. Owner {:?} !=  invoking_program_id {:?}",
                        compressed_account_with_context.owner().to_bytes(),
                        invoking_program_id).as_str()
                    );
                    Err(SystemProgramError::SignerCheckFailed.into())
                }
            },
        )
}

/// Write access check for output compressed accounts.
/// - Only program-owned output accounts can hold data.
/// - Every output account that holds data has to be owned by the
///   invoking_program.
/// - outputs without data can be owned by any pubkey.
#[inline(never)]
#[profile]
pub fn output_compressed_accounts_write_access_check<'a, 'info, T: InstructionData<'a>>(
    inputs: &WrappedInstructionData<'a, T>,
    invoking_program_id: &Pubkey,
) -> Result<()> {
    for compressed_account in inputs.output_accounts() {
        if compressed_account.has_data()
            && *invoking_program_id != compressed_account.owner().to_bytes()
        {
            msg!(
                 format!("Signer/Program cannot write into an account it doesn't own. Write access check failed, compressed account owner {:?} !=  invoking_program_id {:?}.",
                    compressed_account.owner().to_bytes(),
                    invoking_program_id
                ).as_str());
            return Err(SystemProgramError::WriteAccessCheckFailed.into());
        }
        if !compressed_account.has_data()
            && *invoking_program_id == compressed_account.owner().to_bytes()
        {
            msg!("For program owned compressed accounts the data field needs to be defined.");
            return Err(SystemProgramError::DataFieldUndefined.into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use solana_pubkey::Pubkey;

    use super::*;
    #[ignore = "pinocchio doesnt support hashing non solana target os"]
    #[test]
    fn test_cpi_signer_check() {
        for _ in 0..1000 {
            let seeds = [CPI_AUTHORITY_PDA_SEED];
            let invoking_program = Pubkey::new_unique();
            let (derived_signer, bump) =
                Pubkey::find_program_address(&seeds[..], &invoking_program);
            let derived_signer = derived_signer.to_bytes();
            assert_eq!(
                cpi_signer_check(&invoking_program.to_bytes(), &derived_signer, Some(bump)),
                Ok(())
            );
            assert_eq!(
                cpi_signer_check(&invoking_program.to_bytes(), &derived_signer, None),
                Ok(())
            );

            let authority = Pubkey::new_unique().to_bytes();
            let invoking_program = Pubkey::new_unique().to_bytes();
            assert!(
                cpi_signer_check(&invoking_program, &authority, None)
                    == Err(ProgramError::InvalidSeeds)
                    || cpi_signer_check(&invoking_program, &authority, None)
                        == Err(SystemProgramError::CpiSignerCheckFailed.into())
            );
            assert!(
                cpi_signer_check(&invoking_program, &authority, Some(255))
                    == Err(ProgramError::InvalidSeeds)
                    || cpi_signer_check(&invoking_program, &authority, Some(255))
                        == Err(SystemProgramError::CpiSignerCheckFailed.into())
            );
        }
    }
}
