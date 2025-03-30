use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::prelude::*;
use light_compressed_account::instruction_data::zero_copy::{
    ZOutputCompressedAccountWithPackedContext, ZPackedCompressedAccountWithMerkleContext,
};
#[cfg(feature = "bench-sbf")]
use light_heap::{bench_sbf_end, bench_sbf_start};

use crate::errors::SystemProgramError;
/// Checks:
/// 1. Invoking program is signer (cpi_signer_check)
/// 2. Input compressed accounts with data are owned by the invoking program
///    (input_compressed_accounts_signer_check)
/// 3. Output compressed accounts with data are owned by the invoking program
///    (output_compressed_accounts_write_access_check)
pub fn cpi_signer_checks(
    invoking_programid: &Pubkey,
    authority: &Pubkey,
    input_compressed_accounts_with_merkle_context: &[ZPackedCompressedAccountWithMerkleContext],
    output_compressed_accounts: &[ZOutputCompressedAccountWithPackedContext],
) -> Result<()> {
    #[cfg(feature = "bench-sbf")]
    bench_sbf_start!("cpda_cpi_signer_checks");
    // cpi_signer_check(invoking_programid, authority)?;
    #[cfg(feature = "bench-sbf")]
    bench_sbf_end!("cpda_cpi_signer_checks");
    #[cfg(feature = "bench-sbf")]
    bench_sbf_start!("cpd_input_checks");
    input_compressed_accounts_signer_check(
        input_compressed_accounts_with_merkle_context,
        invoking_programid,
    )?;
    #[cfg(feature = "bench-sbf")]
    bench_sbf_end!("cpd_input_checks");
    #[cfg(feature = "bench-sbf")]
    bench_sbf_start!("cpda_cpi_write_checks");
    output_compressed_accounts_write_access_check(output_compressed_accounts, invoking_programid)?;
    #[cfg(feature = "bench-sbf")]
    bench_sbf_end!("cpda_cpi_write_checks");
    Ok(())
}

/// Cpi signer check, validates that the provided invoking program
/// is the actual invoking program.
pub fn cpi_signer_check(invoking_program: &Pubkey, authority: &Pubkey) -> Result<()> {
    let seeds = [CPI_AUTHORITY_PDA_SEED];
    let derived_signer = Pubkey::try_find_program_address(&seeds, invoking_program)
        .ok_or(ProgramError::InvalidSeeds)?
        .0;
    if derived_signer != *authority {
        msg!(
            "Cpi signer check failed. Derived cpi signer {} !=  authority {}",
            derived_signer,
            authority
        );
        return err!(SystemProgramError::CpiSignerCheckFailed);
    }
    Ok(())
}

/// Checks that the invoking program owns all input compressed accounts.
pub fn input_compressed_accounts_signer_check(
    input_compressed_accounts_with_merkle_context: &[ZPackedCompressedAccountWithMerkleContext],
    invoking_program_id: &Pubkey,
) -> Result<()> {
    input_compressed_accounts_with_merkle_context
        .iter()
        .try_for_each(
            |compressed_account_with_context| {
                let invoking_program_id = invoking_program_id.key();
                if invoking_program_id == compressed_account_with_context.compressed_account.owner.into() {
                    Ok(())
                } else {
                    msg!(
                        "Input signer check failed. Program cannot invalidate an account it doesn't own. Owner {:?} !=  invoking_program_id {}",
                        compressed_account_with_context.compressed_account.owner.to_bytes(),
                        invoking_program_id
                    );
                    err!(SystemProgramError::SignerCheckFailed)
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
pub fn output_compressed_accounts_write_access_check(
    output_compressed_accounts: &[ZOutputCompressedAccountWithPackedContext],
    invoking_program_id: &Pubkey,
) -> Result<()> {
    for compressed_account in output_compressed_accounts.iter() {
        if compressed_account.compressed_account.data.is_some()
            && invoking_program_id.key()
                != compressed_account
                    .compressed_account
                    .owner
                    .to_bytes()
                    .into()
        {
            msg!(
                    "Signer/Program cannot write into an account it doesn't own. Write access check failed compressed account owner {:?} !=  invoking_program_id {}",
                    compressed_account.compressed_account.owner.to_bytes(),
                    invoking_program_id.key()
                );
            msg!("compressed_account: {:?}", compressed_account);
            return err!(SystemProgramError::WriteAccessCheckFailed);
        }
        if compressed_account.compressed_account.data.is_none()
            && invoking_program_id.key()
                == compressed_account
                    .compressed_account
                    .owner
                    .to_bytes()
                    .into()
        {
            msg!("For program owned compressed accounts the data field needs to be defined.");
            msg!("compressed_account: {:?}", compressed_account);
            return err!(SystemProgramError::DataFieldUndefined);
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
