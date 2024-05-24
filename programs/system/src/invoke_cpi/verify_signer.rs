use account_compression::{AddressMerkleTreeAccount, StateMerkleTreeAccount};
use anchor_lang::prelude::*;
use light_heap::{bench_sbf_end, bench_sbf_start};
use light_macros::heap_neutral;

use crate::{
    errors::CompressedPdaError, sdk::compressed_account::PackedCompressedAccountWithMerkleContext,
    InstructionDataInvokeCpi,
};

/// Checks:
/// 1. Invoking program is signer (cpi_signer_check)
/// 2. Input compressed accounts with data are owned by the invoking program (input_compressed_accounts_signer_check)
/// 3. Output compressed accounts with data are owned by the invoking program (output_compressed_accounts_write_access_check)
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

/// - If signer seeds are not provided, invoking program is required.
/// - If invoking program is provided signer seeds are required.
/// - If signer seeds are provided, the derived signer has to match the signer.
#[inline(never)]
#[heap_neutral]
pub fn cpi_signer_check(
    signer_seeds: &[Vec<u8>],
    invoking_program: &Pubkey,
    authority: &Pubkey,
) -> Result<()> {
    let seeds = signer_seeds
        .iter()
        .map(|x| x.as_slice())
        .collect::<Vec<&[u8]>>();
    let derived_signer =
        Pubkey::create_program_address(&seeds[..], invoking_program).map_err(ProgramError::from)?;
    if derived_signer != *authority {
        msg!(
            "Cpi signer check failed. Seeds {:?} derived cpi signer {} !=  authority {}",
            seeds,
            derived_signer,
            authority
        );
        return err!(CompressedPdaError::CpiSignerCheckFailed);
    }
    Ok(())
}

/// Checks the signer for input compressed accounts.
/// 1. If a compressed account has data the owner has to be the invokinging program.
/// 2. If a compressed account has data no data the owner has to be authority.
/// (Compressed accounts can be either owned by the program or
/// the authority (which can be a pda) if the compressed account has no data.)
#[inline(never)]
#[heap_neutral]
pub fn input_compressed_accounts_signer_check(
    inputs: &InstructionDataInvokeCpi,
    invoking_program_id: &Pubkey,
    authority: &Pubkey,
) -> Result<()> {
    inputs
    .input_compressed_accounts_with_merkle_context
    .iter()
        .try_for_each(|compressed_account_with_context: &PackedCompressedAccountWithMerkleContext| {

            if compressed_account_with_context.compressed_account.data.is_some()
            {
                // CHECK 1
                let invoking_program_id =invoking_program_id.key();
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
            // CHECK 2
            else if compressed_account_with_context.compressed_account.owner != *authority {
            msg!(
                "signer check failed compressed account owner {} !=  authority {}",
                    compressed_account_with_context.compressed_account.owner,
                    authority
            );
            err!(CompressedPdaError::SignerCheckFailed)
            } else {
                Ok(())
        }
    })?;
    Ok(())
}

/// Checks the write access for output compressed accounts. Only program owned
/// output accounts can hold data. Every output account that holds data has to
/// be owned by the invoking_program. For every account that has data, the owner
/// has to be the invoking_program.
#[inline(never)]
#[heap_neutral]
pub fn output_compressed_accounts_write_access_check(
    inputs: &InstructionDataInvokeCpi,
    invoking_program_id: &Pubkey,
) -> Result<()> {
    for compressed_account in inputs.output_compressed_accounts.iter() {
        if compressed_account.compressed_account.data.is_some()
            && compressed_account.compressed_account.owner != invoking_program_id.key()
        {
            msg!(
                    "Signer/Program cannot write into an account it doesn't own. Write access check failed compressed account owner {} !=  invoking_program_id {}",
                    compressed_account.compressed_account.owner,
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
) -> Result<(u32, Option<u64>, u64)> {
    let merkle_tree =
        AccountLoader::<StateMerkleTreeAccount>::try_from(merkle_tree_acc_info).unwrap();
    let merkle_tree_unpacked = merkle_tree.load()?;
    let seq = merkle_tree_unpacked.load_merkle_tree()?.sequence_number as u64 + 1;
    let next_index: u32 = merkle_tree_unpacked.load_next_index()?.try_into().unwrap();
    let network_fee = if merkle_tree_unpacked.metadata.rollover_metadata.network_fee != 0 {
        Some(merkle_tree_unpacked.metadata.rollover_metadata.network_fee)
    } else {
        None
    };
    // TODO: rename delegate to program_owner
    if merkle_tree_unpacked.metadata.access_metadata.delegate != Pubkey::default() {
        if let Some(invoking_program) = invoking_program {
            if *invoking_program == merkle_tree_unpacked.metadata.access_metadata.delegate {
                return Ok((next_index, network_fee, seq));
            }
        }
        msg!(
            "invoking_program.key() {:?} == merkle_tree_unpacked.delegate {:?}",
            invoking_program,
            merkle_tree_unpacked.metadata.access_metadata.delegate
        );
        return Err(CompressedPdaError::InvalidMerkleTreeOwner.into());
    }
    Ok((next_index, network_fee, seq))
}

pub fn check_program_owner_address_merkle_tree<'a, 'b: 'a>(
    merkle_tree_acc_info: &'b AccountInfo<'a>,
    invoking_program: &Option<Pubkey>,
) -> Result<Option<u64>> {
    let merkle_tree =
        AccountLoader::<AddressMerkleTreeAccount>::try_from(merkle_tree_acc_info).unwrap();
    let merkle_tree_unpacked = merkle_tree.load()?;
    let network_fee = if merkle_tree_unpacked.metadata.rollover_metadata.network_fee != 0 {
        Some(merkle_tree_unpacked.metadata.rollover_metadata.network_fee)
    } else {
        None
    };
    // TODO: rename delegate to program_owner
    if merkle_tree_unpacked.metadata.access_metadata.delegate != Pubkey::default() {
        if let Some(invoking_program) = invoking_program {
            if *invoking_program == merkle_tree_unpacked.metadata.access_metadata.delegate {
                msg!(
                    "invoking_program.key() {:?} == merkle_tree_unpacked.delegate {:?}",
                    invoking_program,
                    merkle_tree_unpacked.metadata.access_metadata.delegate
                );
                return Ok(network_fee);
            }
        }
        msg!(
            "invoking_program.key() {:?} == merkle_tree_unpacked.delegate {:?}",
            invoking_program,
            merkle_tree_unpacked.metadata.access_metadata.delegate
        );
        Err(CompressedPdaError::InvalidMerkleTreeOwner.into())
    } else {
        Ok(network_fee)
    }
}
