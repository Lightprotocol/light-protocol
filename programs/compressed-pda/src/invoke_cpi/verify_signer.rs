use account_compression::{AddressMerkleTreeAccount, StateMerkleTreeAccount};
use anchor_lang::prelude::*;
use light_heap::{bench_sbf_end, bench_sbf_start};
use light_macros::heap_neutral;

use crate::{
    errors::CompressedPdaError, sdk::compressed_account::PackedCompressedAccountWithMerkleContext,
    InstructionDataInvokeCpi,
};

pub fn cpi_signer_checks(
    signer_seeds: &[Vec<u8>],
    invoking_programid: &Pubkey,
    authority: &Pubkey,
    inputs: &InstructionDataInvokeCpi,
) -> Result<()> {
    bench_sbf_start!("cpda_cpi_signer_checks");
    cpi_signer_check(signer_seeds, invoking_programid, authority)?;
    bench_sbf_end!("cpda_cpi_signer_checks");
    bench_sbf_start!("cpd_input_checks");
    input_compressed_accounts_signer_check(inputs, invoking_programid, authority)?;
    bench_sbf_end!("cpd_input_checks");
    bench_sbf_start!("cpda_cpi_write_checks");
    output_compressed_accounts_write_access_check(inputs, invoking_programid)?;
    bench_sbf_end!("cpda_cpi_write_checks");
    Ok(())
}

/// If signer seeds are not provided, invoking program is required.
/// If invoking program is provided signer seeds are required.
/// If signer seeds are provided, the derived signer has to match the signer.
#[inline(never)]
#[heap_neutral]
pub fn cpi_signer_check(
    signer_seeds: &[Vec<u8>],
    invoking_program: &Pubkey,
    signer: &Pubkey,
) -> Result<()> {
    let seeds = signer_seeds
        .iter()
        .map(|x| x.as_slice())
        .collect::<Vec<&[u8]>>();
    let derived_signer =
        Pubkey::create_program_address(&seeds[..], invoking_program).map_err(ProgramError::from)?;
    if derived_signer != *signer {
        msg!(
                    "Signer/Program cannot write into an account it doesn't own. Write access check failed derived cpi signer {} !=  signer {}",
                    signer,
                    signer
                );
        msg!("seeds: {:?}", seeds);
        return err!(CompressedPdaError::SignerCheckFailed);
    }
    Ok(())
}

/// Checks the signer for input compressed accounts.
/// 1. If any compressed account has data the invoking program must be defined.
/// 2. If any compressed account has data the owner has to be the invokinging program.
/// 3. If no compressed account has data the owner has to be the signer.
#[inline(never)]
#[heap_neutral]
pub fn input_compressed_accounts_signer_check(
    inputs: &InstructionDataInvokeCpi,
    invoking_program_id: &Pubkey,
    signer: &Pubkey,
) -> Result<()> {
    inputs
    .input_compressed_accounts_with_merkle_context
    .iter()
        .try_for_each(|compressed_account_with_context: &PackedCompressedAccountWithMerkleContext| {

            if compressed_account_with_context.compressed_account.data.is_some()
            {
                // CHECK 1
                let invoking_program_id =invoking_program_id.key();
                // CHECK 2
                if invoking_program_id != compressed_account_with_context.compressed_account.owner {
                msg!(
                        "Signer/Program cannot read from an account it doesn't own. Read access check failed compressed account owner {} !=  invoking_program_id {}",
                        compressed_account_with_context.compressed_account.owner,
                    invoking_program_id
                );
                    err!(CompressedPdaError::SignerCheckFailed)
                } else {
                    Ok(())
                }
            }
            // CHECK 3
            else if compressed_account_with_context.compressed_account.owner != *signer {
            msg!(
                "signer check failed compressed account owner {} !=  signer {}",
                    compressed_account_with_context.compressed_account.owner,
                    signer
            );
            err!(CompressedPdaError::SignerCheckFailed)
            } else {
                Ok(())
        }
    })?;
    Ok(())
}

/// Checks the write access for output compressed accounts.
/// Only program owned output accounts can hold data.
/// Every output account that holds data has to be owned by the invoking_program.
/// For every account that has data, the owner has to be the invoking_program.
#[inline(never)]
#[heap_neutral]
pub fn output_compressed_accounts_write_access_check(
    inputs: &InstructionDataInvokeCpi,
    invoking_program_id: &Pubkey,
) -> Result<()> {
    // is triggered if one output account has data
    for compressed_account in inputs.output_compressed_accounts.iter() {
        if compressed_account.data.is_some()
            && compressed_account.owner != invoking_program_id.key()
        {
            msg!(
                    "Signer/Program cannot write into an account it doesn't own. Write access check failed compressed account owner {} !=  invoking_program_id {}",
                    compressed_account.owner,
                    invoking_program_id.key()
                );
            msg!("compressed_account: {:?}", compressed_account);
            return err!(CompressedPdaError::WriteAccessCheckFailed);
        }
    }
    Ok(())
}

pub fn check_program_owner_state_merkle_tree<'a, 'b: 'a>(
    merkle_tree_acc_info: &'b AccountInfo<'a>,
    invoking_program: &Option<Pubkey>,
) -> Result<u32> {
    let merkle_tree =
        AccountLoader::<StateMerkleTreeAccount>::try_from(merkle_tree_acc_info).unwrap();
    let merkle_tree_unpacked = merkle_tree.load()?;
    let next_index = merkle_tree_unpacked.load_next_index()?.try_into().unwrap();
    // TODO: rename delegate to program_owner
    if merkle_tree_unpacked.delegate != Pubkey::default() {
        if let Some(invoking_program) = invoking_program {
            if *invoking_program == merkle_tree_unpacked.delegate {
                msg!(
                    "invoking_program.key() {:?} == merkle_tree_unpacked.delegate {:?}",
                    invoking_program,
                    merkle_tree_unpacked.delegate
                );
                return Ok(next_index);
            }
        }
        return Err(CompressedPdaError::InvalidMerkleTreeOwner.into());
    }
    Ok(next_index)
}

pub fn check_program_owner_address_merkle_tree<'a, 'b: 'a>(
    merkle_tree_acc_info: &'b AccountInfo<'a>,
    invoking_program: &Option<Pubkey>,
) -> Result<()> {
    let merkle_tree =
        AccountLoader::<AddressMerkleTreeAccount>::try_from(merkle_tree_acc_info).unwrap();
    let merkle_tree_unpacked = merkle_tree.load()?;
    // TODO: rename delegate to program_owner

    if merkle_tree_unpacked.delegate != Pubkey::default() {
        if let Some(invoking_program) = invoking_program {
            if *invoking_program == merkle_tree_unpacked.delegate {
                msg!(
                    "invoking_program.key() {:?} == merkle_tree_unpacked.delegate {:?}",
                    invoking_program,
                    merkle_tree_unpacked.delegate
                );
                return Ok(());
            }
        }
        msg!(
            "invoking_program.key() {:?} == merkle_tree_unpacked.delegate {:?}",
            invoking_program,
            merkle_tree_unpacked.delegate
        );
        Err(CompressedPdaError::InvalidMerkleTreeOwner.into())
    } else {
        Ok(())
    }
}
