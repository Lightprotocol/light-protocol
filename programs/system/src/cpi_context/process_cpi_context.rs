use light_account_checks::discriminator::Discriminator;
use light_batched_merkle_tree::queue::BatchedQueueAccount;
use light_compressed_account::{
    compressed_account::{CompressedAccountConfig, CompressedAccountDataConfig},
    instruction_data::{
        data::{
            OutputCompressedAccountWithPackedContext,
            OutputCompressedAccountWithPackedContextConfig,
        },
        traits::{InstructionData, OutputAccount},
    },
    pubkey::AsPubkey,
};
use light_program_profiler::profile;
use light_zero_copy::ZeroCopyNew;
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};
use solana_msg::msg;

use super::state::{deserialize_cpi_context_account, ZCpiContextAccount2};
use crate::{
    context::WrappedInstructionData, cpi_context::state::deserialize_cpi_context_account_cleared,
    errors::SystemProgramError, Result,
};

/// Cpi context enables the use of input compressed accounts owned by different
/// programs.
///
/// Example:
/// - a transaction invokes a pda program, which transfers tokens and modifies a
///   compressed pda
/// - the compressed pda is owned by pda program while the
///   compressed token accounts are owned by the compressed token program
///
/// without cpi context:
/// - naively invoking each compressed token via cpi and modifying the pda
///   requires two proofs 128 bytes and ~100,000 CU each
///
/// with cpi context:
/// - only one proof is required -> less instruction data and CU cost
/// 1. first invocation (token program) performs signer checks of the compressed
///    token accounts, caches these in the cpi context and returns. The state
///    transition is not executed yet.
/// 2. second invocation (pda program) performs signer checks of the pda
///    compressed account, reads cpi context and combines the instruction instruction_data
///    with verified instruction_data from the cpi context. The proof is verified and
///    other state transition is executed with the combined instruction_data.
#[profile]
pub fn process_cpi_context<'a, 'info, T: InstructionData<'a>>(
    mut instruction_data: WrappedInstructionData<'a, T>,
    cpi_context_account_info: Option<&'info AccountInfo>,
    fee_payer: Pubkey,
    remaining_accounts: &[AccountInfo],
) -> Result<Option<(usize, WrappedInstructionData<'a, T>)>> {
    let cpi_context = &instruction_data.cpi_context();
    if cpi_context_account_info.is_some() && cpi_context.is_none() {
        msg!("cpi context account is some but cpi context is none");
        return Err(SystemProgramError::CpiContextMissing.into());
    }
    if let Some(cpi_context) = cpi_context {
        let cpi_context_account_info = match cpi_context_account_info {
            Some(cpi_context_account_info) => cpi_context_account_info,
            None => return Err(SystemProgramError::CpiContextAccountUndefined.into()),
        };

        if cpi_context.set_context || cpi_context.first_set_context {
            set_cpi_context(fee_payer, cpi_context_account_info, instruction_data)?;
            return Ok(None);
        } else {
            let cpi_context_account = deserialize_cpi_context_account(cpi_context_account_info)?;
            validate_cpi_context_associated_with_merkle_tree(
                &instruction_data,
                &cpi_context_account,
                remaining_accounts,
            )?;
            if cpi_context_account.is_empty() {
                return Err(SystemProgramError::CpiContextEmpty.into());
            }
            if (*cpi_context_account.fee_payer).to_bytes() != fee_payer {
                msg!(format!(" {:?} != {:?}", fee_payer, cpi_context_account.fee_payer).as_str());
                return Err(SystemProgramError::CpiContextFeePayerMismatch.into());
            }

            instruction_data.set_cpi_context(cpi_context_account)?;
            return Ok(Some((1, instruction_data)));
        }
    }
    Ok(Some((0, instruction_data)))
}

#[profile]
pub fn set_cpi_context<'a, 'info, T: InstructionData<'a>>(
    fee_payer: Pubkey,
    cpi_context_account_info: &'info AccountInfo,
    instruction_data: WrappedInstructionData<'a, T>,
) -> Result<()> {
    // SAFETY Assumptions:
    // -  previous data in cpi_context_account
    //   -> we require the account to be cleared in the beginning of a
    //   transaction
    // - leaf over data: There cannot be any leftover data in the
    //   account since if the transaction fails the account doesn't change.

    // Expected usage:
    // 1. The first invocation is marked with
    // No need to store the proof (except in first invocation),
    // cpi context, compress_or_decompress_lamports,
    // relay_fee
    // 2. Subsequent invocations check the proof and fee payer

    let cpi_context = instruction_data
        .cpi_context()
        .ok_or(SystemProgramError::CpiContextMissing)?;

    if cpi_context.first_set_context {
        let mut cpi_context_account =
            deserialize_cpi_context_account_cleared(cpi_context_account_info)?;
        *cpi_context_account.fee_payer = fee_payer.into();
        cpi_context_account.store_data(&instruction_data)?;
    } else {
        let mut cpi_context_account = deserialize_cpi_context_account(cpi_context_account_info)?;

        if *cpi_context_account.fee_payer == fee_payer && !cpi_context_account.is_empty() {
            cpi_context_account.store_data(&instruction_data)?;
        } else {
            msg!(format!(
                " {:?} != {:?} or cpi context account empty {}",
                fee_payer,
                cpi_context_account.fee_payer,
                cpi_context_account.is_empty()
            )
            .as_str());
            return Err(SystemProgramError::CpiContextFeePayerMismatch.into());
        }
    }

    Ok(())
}

/// Copy CPI context outputs to the provided buffer.
/// This way we ensure that all data involved in the instruction is emitted in this transaction.
/// This prevents an edge case where users misuse the cpi context over multiple transactions
/// and the indexer cannot find all output account data.
#[profile]
pub fn copy_cpi_context_outputs(
    cpi_context_account: &Option<ZCpiContextAccount2<'_>>,
    bytes: &mut [u8],
) -> Result<()> {
    if let Some(cpi_context) = cpi_context_account {
        let (len_store, mut bytes) = bytes.split_at_mut(4);
        len_store.copy_from_slice(
            (cpi_context.out_accounts.len() as u32)
                .to_le_bytes()
                .as_slice(),
        );
        for (output_account, output_data) in cpi_context
            .out_accounts
            .iter()
            .zip(cpi_context.output_data.iter())
        {
            let config = OutputCompressedAccountWithPackedContextConfig {
                compressed_account: CompressedAccountConfig {
                    address: (output_account.address().is_some(), ()),
                    data: (
                        output_account.has_data() || !output_data.is_empty(),
                        CompressedAccountDataConfig {
                            data: output_data.len() as u32,
                        },
                    ),
                },
            };
            let (mut accounts, inner_bytes) =
                OutputCompressedAccountWithPackedContext::new_zero_copy(bytes, config)?;
            if let Some(address) = accounts.compressed_account.address.as_deref_mut() {
                address.copy_from_slice(output_account.address.as_slice());
            }
            accounts.compressed_account.lamports = output_account.lamports;
            accounts.compressed_account.owner = output_account.owner;
            *accounts.merkle_tree_index = output_account.output_merkle_tree_index;
            if let Some(data) = accounts.compressed_account.data.as_mut() {
                data.discriminator = output_account.discriminator;
                *data.data_hash = output_account.data_hash;
                data.data.copy_from_slice(output_data.as_slice());
            }
            bytes = inner_bytes;
        }
    }
    Ok(())
}

#[profile]
fn validate_cpi_context_associated_with_merkle_tree<'a, 'info, T: InstructionData<'a>>(
    instruction_data: &WrappedInstructionData<'a, T>,
    cpi_context_account: &ZCpiContextAccount2<'a>,
    remaining_accounts: &[AccountInfo],
) -> Result<()> {
    let first_merkle_tree_pubkey = if !instruction_data.inputs_empty() {
        let input = instruction_data
            .input_accounts()
            .next()
            .ok_or(SystemProgramError::NoInputs)?;
        let index = input.merkle_context().merkle_tree_pubkey_index;
        // Check bounds before accessing
        if index as usize >= remaining_accounts.len() {
            return Err(SystemProgramError::InvalidAccountIndex.into());
        }
        *remaining_accounts[index as usize].key()
    } else if !instruction_data.outputs_empty() {
        let output = instruction_data
            .output_accounts()
            .next()
            .ok_or(SystemProgramError::NoInputs)?;
        let index = output.merkle_tree_index();
        // Check bounds before accessing
        if index as usize >= remaining_accounts.len() {
            return Err(SystemProgramError::InvalidAccountIndex.into());
        }
        if &remaining_accounts[index as usize].try_borrow_data()?[..8]
            == BatchedQueueAccount::LIGHT_DISCRIMINATOR_SLICE
        {
            let queue_account =
                BatchedQueueAccount::output_from_account_info(&remaining_accounts[index as usize])?;
            queue_account.metadata.associated_merkle_tree.to_bytes()
        } else {
            *remaining_accounts[index as usize].key()
        }
    } else {
        return Err(SystemProgramError::NoInputs.into());
    };

    if *cpi_context_account.associated_merkle_tree != first_merkle_tree_pubkey.to_pubkey_bytes() {
        msg!(format!(
            "first_merkle_tree_pubkey {:?} != associated_merkle_tree {:?}",
            solana_pubkey::Pubkey::new_from_array(first_merkle_tree_pubkey),
            solana_pubkey::Pubkey::new_from_array(
                cpi_context_account.associated_merkle_tree.to_bytes()
            )
        )
        .as_str());
        return Err(SystemProgramError::CpiContextAssociatedMerkleTreeMismatch.into());
    }
    Ok(())
}
