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
use light_zero_copy::ZeroCopyNew;
use pinocchio::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};

use super::state::{deserialize_cpi_context_account, ZCpiContextAccount};
use crate::{context::WrappedInstructionData, errors::SystemProgramError, Result};

/// Diff:
/// 1. return Cpi context instead of combined data.
///
/// Cpi context enables the use of input compressed accounts owned by different
/// programs.
///
/// Example:
/// - a transaction calling a pda program needs to transfer tokens and modify a
///   compressed pda
/// - the pda is owned by pda program while the tokens are owned by the compressed
///   token program
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
        let mut cpi_context_account = deserialize_cpi_context_account(cpi_context_account_info)?;
        // We only validate when executing with the cpi context.
        if !cpi_context.first_set_context && !cpi_context.set_context {
            validate_cpi_context_associated_with_merkle_tree(
                &instruction_data,
                &cpi_context_account,
                remaining_accounts,
            )?;
        }
        if cpi_context.set_context || cpi_context.first_set_context {
            set_cpi_context(fee_payer, cpi_context_account_info, instruction_data)?;
            return Ok(None);
        } else {
            if cpi_context_account.is_empty() {
                return Err(SystemProgramError::CpiContextEmpty.into());
            }
            if (*cpi_context_account.fee_payer).to_bytes() != fee_payer {
                msg!(format!(" {:?} != {:?}", fee_payer, cpi_context_account.fee_payer).as_str());
                return Err(SystemProgramError::CpiContextFeePayerMismatch.into());
            }
            // Zero out the fee payer since the cpi context is being consumed in this instruction.
            *cpi_context_account.fee_payer = Pubkey::default().into();
            instruction_data.set_cpi_context(cpi_context_account)?;
            return Ok(Some((1, instruction_data)));
        }
    }
    Ok(Some((0, instruction_data)))
}
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

    let mut cpi_context_account =
        deserialize_cpi_context_account(cpi_context_account_info).map_err(ProgramError::from)?;

    if instruction_data.cpi_context().unwrap().first_set_context {
        cpi_context_account.clear();
        *cpi_context_account.fee_payer = fee_payer.into();
        cpi_context_account.store_data(&instruction_data)?;
    } else if *cpi_context_account.fee_payer == fee_payer && !cpi_context_account.is_empty() {
        cpi_context_account.store_data(&instruction_data)?;
    } else {
        msg!(format!(" {:?} != {:?}", fee_payer, cpi_context_account.fee_payer).as_str());
        return Err(SystemProgramError::CpiContextFeePayerMismatch.into());
    }

    Ok(())
}
/*
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
    use borsh::{BorshDeserialize, BorshSerialize};
    let cpi_context_account = {
        let data = cpi_context_account_info.try_borrow_data()?;
        let mut cpi_context_account = CpiContextAccount::deserialize(&mut &data[8..])
            .map_err(|_| SystemProgramError::BorrowingDataFailed)?;
        if instruction_data.cpi_context().unwrap().first_set_context {
            cpi_context_account.context.clear();
            cpi_context_account.fee_payer = fee_payer;

            let mut new_cpi_context_data = InstructionDataInvokeCpi::default();
            instruction_data.into_instruction_data_invoke_cpi(&mut new_cpi_context_data);
            cpi_context_account.context.push(new_cpi_context_data);
        } else if cpi_context_account.fee_payer == fee_payer
            && !cpi_context_account.context.is_empty()
        {
            instruction_data.into_instruction_data_invoke_cpi(&mut cpi_context_account.context[0]);
        } else {
            msg!(format!(" {:?} != {:?}", fee_payer, cpi_context_account.fee_payer).as_str());
            return Err(SystemProgramError::CpiContextFeePayerMismatch.into());
        }
        cpi_context_account
    };
    let mut data = cpi_context_account_info.try_borrow_mut_data()?;
    cpi_context_account.serialize(&mut &mut data[8..]).unwrap();
    Ok(())
}
*/
/// Copy CPI context outputs to the provided buffer.
/// This way we ensure that all data involved in the instruction is emitted in this transaction.
/// This prevents an edge case where users misuse the cpi context over multiple transactions
/// and the indexer cannot find all output account data.
pub fn copy_cpi_context_outputs(
    cpi_context_account: &Option<ZCpiContextAccount<'_>>,
    start_offset: usize,
    end_offset: usize,
    cpi_context_account_info: Option<&AccountInfo>,
    cpi_outputs_data_len: usize,
    bytes: &mut [u8],
) -> Result<()> {
    if let Some(cpi_context) = cpi_context_account {
        let (len_store, mut bytes) = bytes.split_at_mut(4);
        len_store.copy_from_slice(
            (cpi_context.out_accounts.len() as u32)
                .to_le_bytes()
                .as_slice(),
        );
        msg!("here");
        for (output_account, output_data) in cpi_context
            .out_accounts
            .iter()
            .zip(cpi_context.output_data.iter())
        {
            let config = OutputCompressedAccountWithPackedContextConfig {
                compressed_account: CompressedAccountConfig {
                    address: (output_account.address().is_some(), ()),
                    data: (
                        !output_data.is_empty(),
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

fn validate_cpi_context_associated_with_merkle_tree<'a, 'info, T: InstructionData<'a>>(
    instruction_data: &WrappedInstructionData<'a, T>,
    cpi_context_account: &ZCpiContextAccount<'a>,
    remaining_accounts: &[AccountInfo],
) -> Result<()> {
    let first_merkle_tree_pubkey = if !instruction_data.inputs_empty() {
        let index = instruction_data
            .input_accounts()
            .next()
            .unwrap()
            .merkle_context()
            .merkle_tree_pubkey_index;
        *remaining_accounts[index as usize].key()
    } else if !instruction_data.outputs_empty() {
        let index = instruction_data
            .output_accounts()
            .next()
            .unwrap()
            .merkle_tree_index();
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

/// Set cpi context tests:
/// 1. Functional: Set cpi context first invocation
/// 2. Functional: Set cpi context subsequent invocation
/// 3. Failing: Set cpi context fee payer mismatch
/// 4. Failing: Set cpi context without first context
///
/// process cpi context:
/// 1. CpiContextMissing
/// 2. CpiContextAccountUndefined
/// 3. NoInputs
/// 4. CpiContextAssociatedMerkleTreeMismatch
/// 5. CpiContextEmpty
/// 6. CpiContextFeePayerMismatch
///
/// Functional process cpi context:
/// 1. Set context
/// 2. Combine (with malicious input in cpi context account)
#[cfg(test)]
mod tests {

    use borsh::BorshSerialize;
    use light_account_checks::account_info::test_account_info::pinocchio::get_account_info;
    use light_compressed_account::{
        compressed_account::{
            CompressedAccount, PackedCompressedAccountWithMerkleContext, PackedMerkleContext,
        },
        instruction_data::{
            cpi_context::CompressedCpiContext, data::OutputCompressedAccountWithPackedContext,
            invoke_cpi::InstructionDataInvokeCpi, traits::instruction_data_eq,
            zero_copy::ZInstructionDataInvokeCpi,
        },
    };
    use light_zero_copy::borsh::Deserialize;
    use pinocchio::pubkey::Pubkey;

    use super::*;
    use crate::cpi_context::state::{cpi_context_account_new, CpiContextAccountInitParams};

    fn clean_input_data(instruction_data: &mut InstructionDataInvokeCpi) {
        instruction_data.cpi_context = None;
        instruction_data.compress_or_decompress_lamports = None;
        instruction_data.relay_fee = None;
        instruction_data.proof = None;
    }

    fn create_test_cpi_context_account(associated_merkle_tree: Option<Pubkey>) -> AccountInfo {
        let associated_merkle_tree =
            associated_merkle_tree.unwrap_or(solana_pubkey::Pubkey::new_unique().to_bytes());
        let params = CpiContextAccountInitParams::new(associated_merkle_tree);
        let account_info = get_account_info(
            solana_pubkey::Pubkey::new_unique().to_bytes(),
            crate::ID,
            false,
            true,
            false,
            vec![0u8; 20000],
        );
        cpi_context_account_new(&account_info, params).unwrap();
        account_info
    }

    fn create_test_instruction_data(
        first_set_context: bool,
        set_context: bool,
        iter: u8,
    ) -> InstructionDataInvokeCpi {
        InstructionDataInvokeCpi {
            proof: None,
            new_address_params: vec![],
            input_compressed_accounts_with_merkle_context: vec![
                PackedCompressedAccountWithMerkleContext {
                    compressed_account: CompressedAccount {
                        owner: solana_pubkey::Pubkey::new_unique().to_bytes().into(),
                        lamports: iter.into(),
                        address: None,
                        data: None,
                    },
                    merkle_context: PackedMerkleContext {
                        merkle_tree_pubkey_index: 0,
                        queue_pubkey_index: iter,
                        leaf_index: 0,
                        prove_by_index: false,
                    },
                    root_index: iter.into(),
                    read_only: false,
                },
            ],
            output_compressed_accounts: vec![OutputCompressedAccountWithPackedContext {
                compressed_account: CompressedAccount {
                    owner: solana_pubkey::Pubkey::new_unique().to_bytes().into(),
                    lamports: iter.into(),
                    address: None,
                    data: None,
                },
                merkle_tree_index: iter,
            }],
            relay_fee: None,
            compress_or_decompress_lamports: None,
            is_compress: false,
            cpi_context: Some(CompressedCpiContext {
                first_set_context,
                set_context,
                cpi_context_account_index: 0,
            }),
        }
    }

    fn get_invalid_merkle_tree_account_info() -> AccountInfo {
        let data = vec![172, 43, 172, 186, 29, 73, 219, 84];
        get_account_info(
            solana_pubkey::Pubkey::new_unique().to_bytes(),
            crate::ID,
            false,
            true,
            false,
            data,
        )
    }

    fn get_merkle_tree_account_info() -> AccountInfo {
        let data = vec![22, 20, 149, 218, 74, 204, 128, 166];
        get_account_info(
            solana_pubkey::Pubkey::new_unique().to_bytes(),
            crate::ID,
            false,
            true,
            false,
            data,
        )
    }

    #[test]
    fn test_set_cpi_context_first_invocation() {
        let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
        let cpi_context_account = create_test_cpi_context_account(None);

        let instruction_data = create_test_instruction_data(true, true, 1);
        let input_bytes = instruction_data.try_to_vec().unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
        let result = set_cpi_context(fee_payer, &cpi_context_account, w_instruction_data);
        // assert
        {
            assert!(result.is_ok());
            let input_bytes = instruction_data.try_to_vec().unwrap();
            let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
            let cpi_context = deserialize_cpi_context_account(&cpi_context_account).unwrap();
            assert_eq!(cpi_context.fee_payer.to_bytes(), fee_payer);
            assert!(instruction_data_eq(&cpi_context, &z_inputs));
        }
    }

    #[test]
    fn test_set_cpi_context_subsequent_invocation() {
        let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
        let cpi_context_account = create_test_cpi_context_account(None);
        let mut first_instruction_data = create_test_instruction_data(true, true, 1);
        // First invocation
        {
            let input_bytes = first_instruction_data.try_to_vec().unwrap();
            let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
            let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
            set_cpi_context(fee_payer, &cpi_context_account, w_instruction_data).unwrap();
        }
        let inputs_subsequent = create_test_instruction_data(false, true, 2);
        let mut input_bytes = Vec::new();
        inputs_subsequent.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
        let result = set_cpi_context(fee_payer, &cpi_context_account, w_instruction_data);
        // assert
        {
            assert!(result.is_ok());
            let input_bytes = inputs_subsequent.try_to_vec().unwrap();
            let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
            let cpi_context = deserialize_cpi_context_account(&cpi_context_account).unwrap();
            assert_eq!(cpi_context.fee_payer.to_bytes(), fee_payer);

            // Create expected instruction data.
            clean_input_data(&mut first_instruction_data);
            first_instruction_data
                .output_compressed_accounts
                .extend(inputs_subsequent.output_compressed_accounts);
            first_instruction_data
                .input_compressed_accounts_with_merkle_context
                .extend(inputs_subsequent.input_compressed_accounts_with_merkle_context);

            let input_bytes = first_instruction_data.try_to_vec().unwrap();
            let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
            //assert_eq!(cpi_context.context[0], z_inputs);
        }
    }

    #[test]
    fn test_set_cpi_context_fee_payer_mismatch() {
        let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
        let cpi_context_account = create_test_cpi_context_account(None);
        let first_instruction_data = create_test_instruction_data(true, true, 1);
        // First invocation
        {
            let input_bytes = first_instruction_data.try_to_vec().unwrap();
            let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
            let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
            set_cpi_context(fee_payer, &cpi_context_account, w_instruction_data).unwrap();
        }

        let different_fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
        let inputs_subsequent = create_test_instruction_data(false, true, 2);
        let mut input_bytes = Vec::new();
        inputs_subsequent.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
        let result = set_cpi_context(
            different_fee_payer,
            &cpi_context_account,
            w_instruction_data,
        );
        assert_eq!(
            result.unwrap_err(),
            SystemProgramError::CpiContextFeePayerMismatch.into()
        );
    }

    #[test]
    fn test_set_cpi_context_without_first_context() {
        let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
        let cpi_context_account = create_test_cpi_context_account(None);
        let inputs_first = create_test_instruction_data(false, true, 1);
        let mut input_bytes = Vec::new();
        inputs_first.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
        let result = set_cpi_context(fee_payer, &cpi_context_account, w_instruction_data);
        assert_eq!(
            result,
            Err(SystemProgramError::CpiContextFeePayerMismatch.into())
        );
    }

    /// Check: process cpi 1
    #[test]
    fn test_process_cpi_context_both_none() {
        let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
        let instruction_data = create_test_instruction_data(false, true, 1);
        let cpi_context_account: Option<&AccountInfo> = None;
        let mut input_bytes = Vec::new();
        instruction_data.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();

        let result = process_cpi_context(w_instruction_data, cpi_context_account, fee_payer, &[])
            .unwrap_err();
        assert_eq!(
            result,
            SystemProgramError::CpiContextAccountUndefined.into()
        );
    }

    /// Check: process cpi 1
    #[test]
    fn test_process_cpi_context_account_none_context_some() {
        let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
        let instruction_data = create_test_instruction_data(false, true, 1);
        let cpi_context_account: Option<&AccountInfo> = None;
        let mut input_bytes = Vec::new();
        instruction_data.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
        let result = process_cpi_context(w_instruction_data, cpi_context_account, fee_payer, &[])
            .unwrap_err();
        assert_eq!(
            result,
            SystemProgramError::CpiContextAccountUndefined.into()
        );
    }

    /// Check: process cpi 2
    #[test]
    fn test_process_cpi_context_account_some_context_none() {
        let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
        let instruction_data = InstructionDataInvokeCpi {
            cpi_context: None,
            ..create_test_instruction_data(false, true, 1)
        };
        let cpi_context_account = create_test_cpi_context_account(None);

        let mut input_bytes = Vec::new();
        instruction_data.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
        let result = process_cpi_context(
            w_instruction_data,
            Some(&cpi_context_account),
            fee_payer,
            &[],
        )
        .unwrap_err();
        assert_eq!(result, SystemProgramError::CpiContextMissing.into());
    }

    /// Check: process cpi 3
    #[test]
    fn test_process_cpi_no_inputs() {
        let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
        let mut instruction_data = create_test_instruction_data(false, false, 1);
        instruction_data.input_compressed_accounts_with_merkle_context = vec![];
        instruction_data.output_compressed_accounts = vec![];
        instruction_data.new_address_params = vec![];

        let merkle_tree_account_info = get_merkle_tree_account_info();
        let cpi_context_account =
            create_test_cpi_context_account(Some(*merkle_tree_account_info.key()));
        let mut input_bytes = Vec::new();
        instruction_data.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
        let result = process_cpi_context(
            w_instruction_data,
            Some(&cpi_context_account),
            fee_payer,
            &[merkle_tree_account_info],
        )
        .unwrap_err();
        assert_eq!(result, SystemProgramError::NoInputs.into());
    }

    /// Check: process cpi 4
    #[test]
    fn test_process_cpi_context_associated_tree_mismatch() {
        let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
        let mut instruction_data = create_test_instruction_data(true, true, 1);
        instruction_data
            .cpi_context
            .as_mut()
            .unwrap()
            .first_set_context = false;
        instruction_data.cpi_context.as_mut().unwrap().set_context = false;
        let cpi_context_account = create_test_cpi_context_account(None);
        let merkle_tree_account_info = get_invalid_merkle_tree_account_info();
        let remaining_accounts = &[merkle_tree_account_info];
        let mut input_bytes = Vec::new();
        instruction_data.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
        let result = process_cpi_context(
            w_instruction_data,
            Some(&cpi_context_account),
            fee_payer,
            remaining_accounts,
        )
        .unwrap_err();
        assert_eq!(
            result,
            SystemProgramError::CpiContextAssociatedMerkleTreeMismatch.into()
        );
    }

    /// Check: process cpi 5
    #[test]
    fn test_process_cpi_context_no_set_context() {
        let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
        let instruction_data = create_test_instruction_data(false, false, 1);
        let merkle_tree_account_info = get_merkle_tree_account_info();
        let cpi_context_account =
            create_test_cpi_context_account(Some(*merkle_tree_account_info.key()));
        let remaining_accounts = &[merkle_tree_account_info];
        let mut input_bytes = Vec::new();
        instruction_data.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
        let result = process_cpi_context(
            w_instruction_data,
            Some(&cpi_context_account),
            fee_payer,
            remaining_accounts,
        )
        .unwrap_err();
        assert_eq!(result, SystemProgramError::CpiContextEmpty.into());
    }

    /// Check: process cpi 6
    #[test]
    fn test_process_cpi_context_empty_context_error() {
        let fee_payer = Pubkey::default();
        let instruction_data = create_test_instruction_data(false, true, 1);
        let merkle_tree_account_info = get_merkle_tree_account_info();
        let cpi_context_account =
            create_test_cpi_context_account(Some(*merkle_tree_account_info.key()));
        let remaining_accounts = &[merkle_tree_account_info];
        let mut input_bytes = Vec::new();
        instruction_data.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
        let result = process_cpi_context(
            w_instruction_data,
            Some(&cpi_context_account),
            fee_payer,
            remaining_accounts,
        )
        .unwrap_err();
        assert_eq!(
            result,
            SystemProgramError::CpiContextFeePayerMismatch.into()
        );
    }

    /// Check: process cpi 6
    #[test]
    fn test_process_cpi_context_fee_payer_mismatch_error() {
        let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
        let instruction_data = create_test_instruction_data(true, true, 1);
        let merkle_tree_account_info = get_merkle_tree_account_info();
        let cpi_context_account =
            create_test_cpi_context_account(Some(*merkle_tree_account_info.key()));
        let remaining_accounts = &[merkle_tree_account_info];
        let mut input_bytes = Vec::new();
        instruction_data.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
        let result = process_cpi_context(
            w_instruction_data,
            Some(&cpi_context_account),
            fee_payer,
            remaining_accounts,
        );
        assert!(result.is_ok());
        let invalid_fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
        let instruction_data = create_test_instruction_data(false, true, 1);
        let mut input_bytes = Vec::new();
        instruction_data.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
        let result = process_cpi_context(
            w_instruction_data,
            Some(&cpi_context_account),
            invalid_fee_payer,
            remaining_accounts,
        )
        .unwrap_err();
        assert_eq!(
            result,
            SystemProgramError::CpiContextFeePayerMismatch.into()
        );
    }

    #[test]
    fn test_process_cpi_context_set_context() {
        let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
        let mut instruction_data = create_test_instruction_data(true, true, 1);
        let merkle_tree_account_info = get_merkle_tree_account_info();
        let cpi_context_account =
            create_test_cpi_context_account(Some(*merkle_tree_account_info.key()));
        let remaining_accounts = &[merkle_tree_account_info];
        let mut input_bytes = Vec::new();
        instruction_data.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
        let result = process_cpi_context(
            w_instruction_data,
            Some(&cpi_context_account),
            fee_payer,
            remaining_accounts,
        );
        // assert
        {
            assert!(result.is_ok());

            let cpi_context = deserialize_cpi_context_account(&cpi_context_account).unwrap();

            // Create expected instruction data.
            clean_input_data(&mut instruction_data);
            let input_bytes = instruction_data.try_to_vec().unwrap();
            let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
            //assert_eq!(cpi_context.context[0], z_inputs);
            assert!(result.unwrap().is_none());
        }
    }
    /*
    #[test]
    fn test_process_cpi_context_combine() {
        let fee_payer = solana_pubkey::Pubkey::new_unique().to_bytes();
        let mut instruction_data = create_test_instruction_data(true, true, 1);
        let malicious_inputs = create_test_instruction_data(true, true, 100);
        let merkle_tree_account_info = get_merkle_tree_account_info();
        let merkle_tree_pubkey = *merkle_tree_account_info.key();
        let cpi_context_account = create_test_cpi_context_account(Some(merkle_tree_pubkey));
        // Inject data into cpi context account.
        {
            let cpi_context_content = CpiContextAccount {
                fee_payer: Pubkey::default(),
                associated_merkle_tree: merkle_tree_pubkey,
                context: vec![malicious_inputs],
            };
            let input_data = cpi_context_content.try_to_vec().unwrap();
            let input_data_len = input_data.len();
            let mut data = cpi_context_account.try_borrow_mut_data().unwrap();

            data[8 + 64..input_data_len + 8 + 64].copy_from_slice(&input_data);
        }

        let remaining_accounts = &[merkle_tree_account_info];
        let mut input_bytes = Vec::new();
        instruction_data.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
        let result = process_cpi_context(
            w_instruction_data,
            Some(&cpi_context_account),
            fee_payer,
            remaining_accounts,
        );
        {
            assert!(result.is_ok());
            let (cpi_context, _) = deserialize_cpi_context_account(&cpi_context_account).unwrap();
            // Create expected instruction data.
            clean_input_data(&mut instruction_data);
            let input_bytes = instruction_data.try_to_vec().unwrap();
            let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
            assert_eq!(cpi_context.context[0], z_inputs);
            assert_eq!(
                cpi_context.associated_merkle_tree.to_bytes(),
                merkle_tree_pubkey
            );
            assert!(result.unwrap().is_none());
        }

        for i in 2..10 {
            let inputs_subsequent = create_test_instruction_data(false, true, i);
            let mut input_bytes = Vec::new();
            inputs_subsequent.serialize(&mut input_bytes).unwrap();
            let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
            let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();
            let result = process_cpi_context(
                w_instruction_data,
                Some(&cpi_context_account),
                fee_payer,
                remaining_accounts,
            );
            // assert
            {
                assert!(result.is_ok());
                let input_bytes = inputs_subsequent.try_to_vec().unwrap();
                let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
                let (cpi_context, _) =
                    deserialize_cpi_context_account(&cpi_context_account).unwrap();
                assert_eq!(cpi_context.fee_payer.to_bytes(), fee_payer);
                assert_eq!(cpi_context.context.len(), 1);
                assert_ne!(cpi_context.context[0], z_inputs);
                instruction_data
                    .output_compressed_accounts
                    .extend(inputs_subsequent.output_compressed_accounts);
                instruction_data
                    .input_compressed_accounts_with_merkle_context
                    .extend(inputs_subsequent.input_compressed_accounts_with_merkle_context);

                let input_bytes = instruction_data.try_to_vec().unwrap();
                let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
                assert_eq!(cpi_context.context[0], z_inputs);
            }
        }

        let instruction_data = create_test_instruction_data(false, false, 10);
        let mut input_bytes = Vec::new();
        instruction_data.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();

        let result = process_cpi_context(
            w_instruction_data,
            Some(&cpi_context_account),
            fee_payer,
            remaining_accounts,
        );
        assert!(result.is_ok());
        let (_, result) = result.unwrap().unwrap();

        assert!(result.new_addresses().next().is_none());

        let mut outputs = result.output_accounts();
        assert_eq!(outputs.next().unwrap().lamports(), 10);

        let mut inputs = result.input_accounts();
        assert_eq!(inputs.next().unwrap().lamports(), 10);

        for ((i, in_account), out_account) in ((1..10).zip(outputs)).zip(inputs) {
            assert_eq!(out_account.lamports(), i as u64);
            assert_eq!(in_account.lamports(), i as u64);
        }

        clear_cpi_context_account(Some(&cpi_context_account)).unwrap();
        let (cpi_context, _) = deserialize_cpi_context_account(&cpi_context_account).unwrap();

        assert_eq!(
            cpi_context.associated_merkle_tree.to_bytes(),
            merkle_tree_pubkey
        );
        assert_eq!(cpi_context.fee_payer.to_bytes(), Pubkey::default());
        assert_eq!(cpi_context.context.len(), 0);
    }*/
}
