use std::mem;

use account_compression::{
    errors::AccountCompressionErrorCode, utils::constants::CPI_AUTHORITY_PDA_SEED,
    AddressMerkleTreeAccount, StateMerkleTreeAccount,
};
use anchor_lang::{prelude::*, Discriminator};
use light_batched_merkle_tree::{
    merkle_tree::{BatchedMerkleTreeAccount, ZeroCopyBatchedMerkleTreeAccount},
    queue::{BatchedQueueAccount, ZeroCopyBatchedQueueAccount},
};
use light_concurrent_merkle_tree::zero_copy::ConcurrentMerkleTreeZeroCopy;
use light_hasher::{Discriminator as LightDiscriminator, Poseidon};
use light_heap::{bench_sbf_end, bench_sbf_start};
use light_macros::heap_neutral;

use crate::{
    errors::SystemProgramError, sdk::compressed_account::PackedCompressedAccountWithMerkleContext,
    OutputCompressedAccountWithPackedContext,
};

/// Checks:
/// 1. Invoking program is signer (cpi_signer_check)
/// 2. Input compressed accounts with data are owned by the invoking program
///    (input_compressed_accounts_signer_check)
/// 3. Output compressed accounts with data are owned by the invoking program
///    (output_compressed_accounts_write_access_check)
pub fn cpi_signer_checks(
    invoking_programid: &Pubkey,
    authority: &Pubkey,
    input_compressed_accounts_with_merkle_context: &[PackedCompressedAccountWithMerkleContext],
    output_compressed_accounts: &[OutputCompressedAccountWithPackedContext],
) -> Result<()> {
    bench_sbf_start!("cpda_cpi_signer_checks");
    cpi_signer_check(invoking_programid, authority)?;
    bench_sbf_end!("cpda_cpi_signer_checks");
    bench_sbf_start!("cpd_input_checks");
    input_compressed_accounts_signer_check(
        input_compressed_accounts_with_merkle_context,
        invoking_programid,
    )?;
    bench_sbf_end!("cpd_input_checks");
    bench_sbf_start!("cpda_cpi_write_checks");
    output_compressed_accounts_write_access_check(output_compressed_accounts, invoking_programid)?;
    bench_sbf_end!("cpda_cpi_write_checks");
    Ok(())
}

/// Cpi signer check, validates that the provided invoking program
/// is the actual invoking program.
#[heap_neutral]
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
    input_compressed_accounts_with_merkle_context: &[PackedCompressedAccountWithMerkleContext],
    invoking_program_id: &Pubkey,
) -> Result<()> {
    input_compressed_accounts_with_merkle_context
        .iter()
        .try_for_each(
            |compressed_account_with_context: &PackedCompressedAccountWithMerkleContext| {
                let invoking_program_id = invoking_program_id.key();
                if invoking_program_id == compressed_account_with_context.compressed_account.owner {
                    Ok(())
                } else {
                    msg!(
                        "Input signer check failed. Program cannot invalidate an account it doesn't own. Owner {} !=  invoking_program_id {}",
                        compressed_account_with_context.compressed_account.owner,
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
    output_compressed_accounts: &[OutputCompressedAccountWithPackedContext],
    invoking_program_id: &Pubkey,
) -> Result<()> {
    for compressed_account in output_compressed_accounts.iter() {
        if compressed_account.compressed_account.data.is_some()
            && compressed_account.compressed_account.owner != invoking_program_id.key()
        {
            msg!(
                    "Signer/Program cannot write into an account it doesn't own. Write access check failed compressed account owner {} !=  invoking_program_id {}",
                    compressed_account.compressed_account.owner,
                    invoking_program_id.key()
                );
            msg!("compressed_account: {:?}", compressed_account);
            return err!(SystemProgramError::WriteAccessCheckFailed);
        }
        if compressed_account.compressed_account.data.is_none()
            && compressed_account.compressed_account.owner == invoking_program_id.key()
        {
            msg!("For program owned compressed accounts the data field needs to be defined.");
            msg!("compressed_account: {:?}", compressed_account);
            return err!(SystemProgramError::DataFieldUndefined);
        }
    }
    Ok(())
}

pub fn check_program_owner_state_merkle_tree<'a, 'b: 'a>(
    merkle_tree_acc_info: &'b AccountInfo<'a>,
    invoking_program: &Option<Pubkey>,
) -> Result<(u32, Option<u64>, u64, Pubkey)> {
    let (seq, next_index, network_fee, program_owner, merkle_tree_pubkey) = {
        let mut discriminator_bytes = [0u8; 8];
        discriminator_bytes.copy_from_slice(&merkle_tree_acc_info.try_borrow_data()?[0..8]);
        msg!("discriminator_bytes: {:?}", discriminator_bytes);
        msg!("pubkey {:?}", merkle_tree_acc_info.key());
        match discriminator_bytes {
            StateMerkleTreeAccount::DISCRIMINATOR => {
                let (seq, next_index) = {
                    let merkle_tree = merkle_tree_acc_info.try_borrow_mut_data()?;
                    let merkle_tree =
                        ConcurrentMerkleTreeZeroCopy::<Poseidon, 26>::from_bytes_zero_copy(
                            &merkle_tree[8 + mem::size_of::<StateMerkleTreeAccount>()..],
                        )
                        .map_err(ProgramError::from)?;

                    let seq = merkle_tree.sequence_number() as u64 + 1;
                    let next_index: u32 = merkle_tree.next_index().try_into().unwrap();
                    (seq, next_index)
                };
                let merkle_tree =
                    AccountLoader::<StateMerkleTreeAccount>::try_from(merkle_tree_acc_info)
                        .unwrap();
                let merkle_tree_unpacked = merkle_tree.load()?;
                (
                    seq,
                    next_index,
                    merkle_tree_unpacked.metadata.rollover_metadata.network_fee,
                    merkle_tree_unpacked.metadata.access_metadata.program_owner,
                    merkle_tree_acc_info.key(),
                )
            }
            BatchedMerkleTreeAccount::DISCRIMINATOR => {
                let merkle_tree =
                    ZeroCopyBatchedMerkleTreeAccount::state_tree_from_account_info_mut(
                        merkle_tree_acc_info,
                    )
                    .map_err(ProgramError::from)?;
                let account = merkle_tree.get_account();
                let seq = account.sequence_number + 1;
                let next_index: u32 = account.next_index.try_into().unwrap();

                (
                    seq,
                    next_index,
                    account.metadata.rollover_metadata.network_fee,
                    account.metadata.access_metadata.program_owner,
                    merkle_tree_acc_info.key(),
                )
            }
            BatchedQueueAccount::DISCRIMINATOR => {
                let merkle_tree = ZeroCopyBatchedQueueAccount::output_queue_from_account_info_mut(
                    merkle_tree_acc_info,
                )
                .map_err(ProgramError::from)?;
                let account = merkle_tree.get_account();
                let seq = u64::MAX;
                let next_index: u32 = account.next_index.try_into().unwrap();

                (
                    seq,
                    next_index,
                    account.metadata.rollover_metadata.network_fee,
                    account.metadata.access_metadata.program_owner,
                    account.metadata.associated_merkle_tree,
                )
            }
            _ => {
                return err!(
                    AccountCompressionErrorCode::StateMerkleTreeAccountDiscriminatorMismatch
                );
            }
        }
    };

    let network_fee = if network_fee != 0 {
        Some(network_fee)
    } else {
        None
    };
    if program_owner != Pubkey::default() {
        if let Some(invoking_program) = invoking_program {
            if *invoking_program == program_owner {
                return Ok((next_index, network_fee, seq, merkle_tree_pubkey));
            }
        }
        msg!(
            "invoking_program.key() {:?} == merkle_tree_unpacked.program_owner {:?}",
            invoking_program,
            program_owner
        );
        return err!(SystemProgramError::InvalidMerkleTreeOwner);
    }
    Ok((next_index, network_fee, seq, merkle_tree_pubkey))
}

// TODO: extend to match batched trees
pub fn check_program_owner_address_merkle_tree<'a, 'b: 'a>(
    merkle_tree_acc_info: &'b AccountInfo<'a>,
    invoking_program: &Option<Pubkey>,
) -> Result<Option<u64>> {
    let discriminator_bytes = merkle_tree_acc_info.try_borrow_data()?[0..8]
        .try_into()
        .unwrap();

    let metadata = match discriminator_bytes {
        AddressMerkleTreeAccount::DISCRIMINATOR => {
            let merkle_tree =
                AccountLoader::<AddressMerkleTreeAccount>::try_from(merkle_tree_acc_info).unwrap();
            let merkle_tree_unpacked = merkle_tree.load()?;
            merkle_tree_unpacked.metadata
        }
        BatchedMerkleTreeAccount::DISCRIMINATOR => {
            let merkle_tree = ZeroCopyBatchedMerkleTreeAccount::address_tree_from_account_info_mut(
                merkle_tree_acc_info,
            )
            .map_err(ProgramError::from)?;
            let account = merkle_tree.get_account();
            account.metadata
        }
        _ => {
            return err!(
                AccountCompressionErrorCode::AddressMerkleTreeAccountDiscriminatorMismatch
            );
        }
    };

    let network_fee = if metadata.rollover_metadata.network_fee != 0 {
        Some(metadata.rollover_metadata.network_fee)
    } else {
        None
    };

    if metadata.access_metadata.program_owner != Pubkey::default() {
        if let Some(invoking_program) = invoking_program {
            if *invoking_program == metadata.access_metadata.program_owner {
                msg!(
                    "invoking_program.key() {:?} == merkle_tree_unpacked.program_owner {:?}",
                    invoking_program,
                    metadata.access_metadata.program_owner
                );
                return Ok(network_fee);
            }
        }
        msg!(
            "invoking_program.key() {:?} == merkle_tree_unpacked.program_owner {:?}",
            invoking_program,
            metadata.access_metadata.program_owner
        );
        err!(SystemProgramError::InvalidMerkleTreeOwner)
    } else {
        Ok(network_fee)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::sdk::compressed_account::{CompressedAccount, CompressedAccountData};

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

    #[test]
    fn test_input_compressed_accounts_signer_check() {
        let authority = Pubkey::new_unique();
        let mut compressed_account_with_context = PackedCompressedAccountWithMerkleContext {
            compressed_account: CompressedAccount {
                owner: authority,
                ..CompressedAccount::default()
            },
            ..PackedCompressedAccountWithMerkleContext::default()
        };

        assert_eq!(
            input_compressed_accounts_signer_check(
                &[compressed_account_with_context.clone()],
                &authority
            ),
            Ok(())
        );

        compressed_account_with_context.compressed_account.owner = Pubkey::new_unique();
        assert_eq!(
            input_compressed_accounts_signer_check(&[compressed_account_with_context], &authority),
            Err(SystemProgramError::SignerCheckFailed.into())
        );
    }

    #[test]
    fn test_output_compressed_accounts_write_access_check() {
        let authority = Pubkey::new_unique();
        let compressed_account = CompressedAccount {
            owner: authority,
            data: Some(CompressedAccountData::default()),
            ..CompressedAccount::default()
        };
        let output_compressed_account = OutputCompressedAccountWithPackedContext {
            compressed_account,
            ..OutputCompressedAccountWithPackedContext::default()
        };

        assert_eq!(
            output_compressed_accounts_write_access_check(&[output_compressed_account], &authority),
            Ok(())
        );

        // Invalid program owner but no data should succeed
        let compressed_account = CompressedAccount {
            owner: Pubkey::new_unique(),
            ..CompressedAccount::default()
        };
        let mut output_compressed_account = OutputCompressedAccountWithPackedContext {
            compressed_account,
            ..OutputCompressedAccountWithPackedContext::default()
        };

        assert_eq!(
            output_compressed_accounts_write_access_check(
                &[output_compressed_account.clone()],
                &authority
            ),
            Ok(())
        );

        // Invalid program owner and data should fail
        output_compressed_account.compressed_account.data = Some(CompressedAccountData::default());

        assert_eq!(
            output_compressed_accounts_write_access_check(&[output_compressed_account], &authority),
            Err(SystemProgramError::WriteAccessCheckFailed.into())
        );
    }
}
