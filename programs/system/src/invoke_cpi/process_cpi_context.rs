use light_account_checks::discriminator::Discriminator;
use light_batched_merkle_tree::queue::BatchedQueueAccount;
use light_compressed_account::instruction_data::{
    invoke_cpi::InstructionDataInvokeCpi, traits::InstructionData,
};
use pinocchio::{account_info::AccountInfo, msg, pubkey::Pubkey};

use super::account::{deserialize_cpi_context_account, CpiContextAccount, ZCpiContextAccount};
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
///    compressed account, reads cpi context and combines the instruction inputs
///    with verified inputs from the cpi context. The proof is verified and
///    other state transition is executed with the combined inputs.
pub fn process_cpi_context<'a, 'info, T: InstructionData<'a>>(
    mut inputs: WrappedInstructionData<'a, T>,
    cpi_context_account_info: Option<&'info AccountInfo>,
    fee_payer: Pubkey,
    remaining_accounts: &[AccountInfo],
) -> Result<Option<(usize, WrappedInstructionData<'a, T>)>> {
    let cpi_context = &inputs.cpi_context();
    if cpi_context_account_info.is_some() && cpi_context.is_none() {
        msg!("cpi context account is some but cpi context is none");
        return Err(SystemProgramError::CpiContextMissing.into());
    }
    if let Some(cpi_context) = cpi_context {
        msg!("cpi context is some");
        let cpi_context_account_info = match cpi_context_account_info {
            Some(cpi_context_account_info) => cpi_context_account_info,
            None => return Err(SystemProgramError::CpiContextAccountUndefined.into()),
        };
        let (mut cpi_context_account, outputs_offsets) =
            deserialize_cpi_context_account(cpi_context_account_info)?;
        let first_merkle_tree_pubkey = if !inputs.inputs_empty() {
            let index = inputs
                .input_accounts()
                .next()
                .unwrap()
                .merkle_context()
                .merkle_tree_pubkey_index;
            *remaining_accounts[index as usize].key()
        } else if !inputs.outputs_empty() {
            let index = inputs.output_accounts().next().unwrap().merkle_tree_index();
            if &remaining_accounts[index as usize].try_borrow_data()?[..8]
                == BatchedQueueAccount::DISCRIMINATOR_SLICE
            {
                let queue_account = BatchedQueueAccount::output_from_account_info(
                    &remaining_accounts[index as usize],
                )?;
                queue_account.metadata.associated_merkle_tree.to_bytes()
            } else {
                *remaining_accounts[index as usize].key()
            }
        } else {
            return Err(SystemProgramError::NoInputs.into());
        };

        if *cpi_context_account.associated_merkle_tree != first_merkle_tree_pubkey.into() {
            msg!(format!(
                "first_merkle_tree_pubkey {:?} != associated_merkle_tree {:?}",
                first_merkle_tree_pubkey, cpi_context_account.associated_merkle_tree
            )
            .as_str());
            return Err(SystemProgramError::CpiContextAssociatedMerkleTreeMismatch.into());
        }
        msg!(format!("cpi_context {:?}", cpi_context).as_str());
        // if cpi_context.first_set_context {
        //     set_cpi_context(fee_payer, cpi_context_account_info, inputs)?;
        //     return Ok(None);
        // } else
        if cpi_context.set_context || cpi_context.first_set_context {
            set_cpi_context(fee_payer, cpi_context_account_info, inputs)?;
            return Ok(None);
        } else {
            if cpi_context_account.context.is_empty() {
                return Err(SystemProgramError::CpiContextEmpty.into());
            }
            if (*cpi_context_account.fee_payer).to_bytes() != fee_payer {
                msg!(format!(" {:?} != {:?}", fee_payer, cpi_context_account.fee_payer).as_str());
                return Err(SystemProgramError::CpiContextFeePayerMismatch.into());
            }
            // Zero out the fee payer since the cpi context is being consumed in this instruction.
            *cpi_context_account.fee_payer = Pubkey::default().into();
            inputs.set_cpi_context(cpi_context_account, outputs_offsets.0, outputs_offsets.1);
            return Ok(Some((1, inputs)));
        }
    } else {
        msg!("cpi context is none");
    }
    Ok(Some((0, inputs)))
}

pub fn set_cpi_context<'a, 'info, T: InstructionData<'a>>(
    fee_payer: Pubkey,
    cpi_context_account_info: &'info AccountInfo,
    inputs: WrappedInstructionData<'a, T>,
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
        let mut cpi_context_account = CpiContextAccount::deserialize(&mut &data[8..]).unwrap();
        if inputs.cpi_context().unwrap().first_set_context {
            msg!("First invocation");
            cpi_context_account.fee_payer = fee_payer;
            cpi_context_account.context.clear();
            msg!("First invocation1");

            let mut instruction_data = InstructionDataInvokeCpi::default();
            inputs.into_instruction_data_invoke_cpi(&mut instruction_data);
            cpi_context_account.context.push(instruction_data);
            msg!("wrapped up first invocation");
        } else if cpi_context_account.fee_payer == fee_payer
            && !cpi_context_account.context.is_empty()
        {
            msg!(format!(
                " cpi context fee payer {:?} != {:?}, is empty {}",
                fee_payer,
                cpi_context_account.fee_payer,
                cpi_context_account.context.is_empty()
            )
            .as_str());
            inputs.into_instruction_data_invoke_cpi(&mut cpi_context_account.context[0]);
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
        let num_outputs: u32 = cpi_context.context[0]
            .output_compressed_accounts
            .len()
            .try_into()
            .unwrap();
        msg!(format!("copy_cpi_context_outputs bytes: {:?}", bytes[..64].to_vec()).as_str());
        let cpi_context_data = cpi_context_account_info.unwrap().try_borrow_data()?;

        msg!(format!(
            "cpi_context_data[start_offset..end_offset] bytes: {:?}",
            cpi_context_data[start_offset..end_offset].to_vec()
        )
        .as_str());
        bytes[0..4].copy_from_slice(num_outputs.to_le_bytes().as_slice());
        bytes[4..4 + cpi_outputs_data_len]
            .copy_from_slice(&cpi_context_data[start_offset..end_offset]);
    }
    Ok(())
}

// /// Set cpi context tests:
// /// 1. Functional: Set cpi context first invocation
// /// 2. Functional: Set cpi context subsequent invocation
// /// 3. Failing: Set cpi context fee payer mismatch
// /// 4. Failing: Set cpi context without first context
// ///
// /// process cpi context:
// /// 1. CpiContextMissing
// /// 2. CpiContextAccountUndefined
// /// 3. NoInputs
// /// 4. CpiContextAssociatedMerkleTreeMismatch
// /// 5. CpiContextEmpty
// /// 6. CpiContextFeePayerMismatch
// ///
// /// Functional process cpi context:
// /// 1. Set context
// /// 2. Combine (with malicious input in cpi context account)
// #[cfg(test)]
// mod tests {
//     use std::cell::RefCell;

//     use light_compressed_account::{
//         compressed_account::{
//             CompressedAccount, PackedCompressedAccountWithMerkleContext, PackedMerkleContext,
//         },
//         instruction_data::{
//             cpi_context::CompressedCpiContext, data::OutputCompressedAccountWithPackedContext,
//             invoke_cpi::InstructionDataInvokeCpi,
//         },
//     };
//     use light_zero_copy::borsh::Deserialize;
//     use pinocchio::pubkey::Pubkey;

//     use super::*;

//     fn clean_input_data(inputs: &mut InstructionDataInvokeCpi) {
//         inputs.cpi_context = None;
//         inputs.compress_or_decompress_lamports = None;
//         inputs.relay_fee = None;
//         inputs.proof = None;
//     }

//     fn create_test_cpi_context_account() -> CpiContextAccount {
//         CpiContextAccount {
//             fee_payer: Pubkey::new_unique(),
//             associated_merkle_tree: Pubkey::new_unique(),
//             context: vec![],
//         }
//     }

//     fn create_test_instruction_data(
//         first_set_context: bool,
//         set_context: bool,
//         iter: u8,
//     ) -> InstructionDataInvokeCpi {
//         InstructionDataInvokeCpi {
//             proof: None,
//             new_address_params: vec![],
//             input_compressed_accounts_with_merkle_context: vec![
//                 PackedCompressedAccountWithMerkleContext {
//                     compressed_account: CompressedAccount {
//                         owner: Pubkey::new_unique(),
//                         lamports: iter.into(),
//                         address: None,
//                         data: None,
//                     },
//                     merkle_context: PackedMerkleContext {
//                         merkle_tree_pubkey_index: 0,
//                         nullifier_queue_pubkey_index: iter,
//                         leaf_index: 0,
//                         prove_by_index: false,
//                     },
//                     root_index: iter.into(),
//                     read_only: false,
//                 },
//             ],
//             output_compressed_accounts: vec![OutputCompressedAccountWithPackedContext {
//                 compressed_account: CompressedAccount {
//                     owner: Pubkey::new_unique(),
//                     lamports: iter.into(),
//                     address: None,
//                     data: None,
//                 },
//                 merkle_tree_index: iter,
//             }],
//             relay_fee: None,
//             compress_or_decompress_lamports: None,
//             is_compress: false,
//             cpi_context: Some(CompressedCpiContext {
//                 first_set_context,
//                 set_context,
//                 cpi_context_account_index: 0,
//             }),
//         }
//     }

//     #[test]
//     fn test_set_cpi_context_first_invocation() {
//         let fee_payer = Pubkey::new_unique();
//         let mut cpi_context_account = create_test_cpi_context_account();
//         let mut inputs = create_test_instruction_data(true, true, 1);
//         let mut input_bytes = Vec::new();
//         inputs.serialize(&mut input_bytes).unwrap();
//         let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();

//         let result = set_cpi_context(fee_payer, &mut cpi_context_account, z_inputs);
//         assert!(result.is_ok());
//         assert_eq!(cpi_context_account.fee_payer, fee_payer);
//         assert_eq!(cpi_context_account.context.len(), 1);
//         assert_ne!(cpi_context_account.context[0], inputs);
//         clean_input_data(&mut inputs);
//         assert_eq!(cpi_context_account.context[0], inputs);
//     }

//     #[test]
//     fn test_set_cpi_context_subsequent_invocation() {
//         let fee_payer = Pubkey::new_unique();
//         let mut cpi_context_account = create_test_cpi_context_account();
//         let inputs_first = create_test_instruction_data(true, true, 1);
//         let mut input_bytes = Vec::new();
//         inputs_first.serialize(&mut input_bytes).unwrap();
//         let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();

//         set_cpi_context(fee_payer, &mut cpi_context_account, z_inputs).unwrap();

//         let mut inputs_subsequent = create_test_instruction_data(false, true, 2);
//         let mut input_bytes = Vec::new();
//         inputs_subsequent.serialize(&mut input_bytes).unwrap();
//         let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
//         let result = set_cpi_context(fee_payer, &mut cpi_context_account, z_inputs);
//         assert!(result.is_ok());
//         assert_eq!(cpi_context_account.context.len(), 2);
//         clean_input_data(&mut inputs_subsequent);
//         assert_eq!(cpi_context_account.context[1], inputs_subsequent);
//     }

//     #[test]
//     fn test_set_cpi_context_fee_payer_mismatch() {
//         let fee_payer = Pubkey::new_unique();
//         let mut cpi_context_account = create_test_cpi_context_account();
//         let inputs_first = create_test_instruction_data(true, true, 1);
//         let mut input_bytes = Vec::new();
//         inputs_first.serialize(&mut input_bytes).unwrap();
//         let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
//         set_cpi_context(fee_payer, &mut cpi_context_account, z_inputs).unwrap();

//         let different_fee_payer = Pubkey::new_unique();
//         let inputs_subsequent = create_test_instruction_data(false, true, 2);
//         let mut input_bytes = Vec::new();
//         inputs_subsequent.serialize(&mut input_bytes).unwrap();
//         let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
//         let result = set_cpi_context(different_fee_payer, &mut cpi_context_account, z_inputs);
//         assert_eq!(
//             result.unwrap_err(),
//             SystemProgramError::CpiContextFeePayerMismatch.into()
//         );
//     }

//     #[test]
//     fn test_set_cpi_context_without_first_context() {
//         let fee_payer = Pubkey::new_unique();
//         let mut cpi_context_account = create_test_cpi_context_account();
//         let inputs_first = create_test_instruction_data(false, true, 1);
//         let mut input_bytes = Vec::new();
//         inputs_first.serialize(&mut input_bytes).unwrap();
//         let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
//         let result = set_cpi_context(fee_payer, &mut cpi_context_account, z_inputs);
//         assert_eq!(
//             result,
//             Err(SystemProgramError::CpiContextFeePayerMismatch.into())
//         );
//     }

//     /// Check: process cpi 1
//     #[test]
//     fn test_process_cpi_context_both_none() {
//         let fee_payer = Pubkey::new_unique();
//         let inputs = create_test_instruction_data(false, true, 1);
//         let mut cpi_context_account: Option<Account<CpiContextAccount>> = None;
//         let mut input_bytes = Vec::new();
//         inputs.serialize(&mut input_bytes).unwrap();
//         let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
//         let result = process_cpi_context(z_inputs, &mut cpi_context_account, fee_payer, &[]);
//         assert_eq!(
//             result,
//             Err(SystemProgramError::CpiContextAccountUndefined.into())
//         );
//     }

//     /// Check: process cpi 1
//     #[test]
//     fn test_process_cpi_context_account_none_context_some() {
//         let fee_payer = Pubkey::new_unique();
//         let inputs = create_test_instruction_data(false, true, 1);
//         let mut cpi_context_account: Option<Account<CpiContextAccount>> = None;
//         let mut input_bytes = Vec::new();
//         inputs.serialize(&mut input_bytes).unwrap();
//         let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
//         let result = process_cpi_context(z_inputs, &mut cpi_context_account, fee_payer, &[]);
//         assert_eq!(
//             result,
//             Err(SystemProgramError::CpiContextAccountUndefined.into())
//         );
//     }

//     /// Check: process cpi 2
//     #[test]
//     fn test_process_cpi_context_account_some_context_none() {
//         let fee_payer = Pubkey::new_unique();
//         let inputs = InstructionDataInvokeCpi {
//             cpi_context: None,
//             ..create_test_instruction_data(false, true, 1)
//         };
//         let mut lamports = 0;
//         let cpi_context_content = CpiContextAccount {
//             fee_payer: Pubkey::default(),
//             associated_merkle_tree: Pubkey::new_unique(),
//             context: vec![],
//         };
//         let mut data = vec![22, 20, 149, 218, 74, 204, 128, 166];
//         data.extend_from_slice(&cpi_context_content.try_to_vec().unwrap());
//         let account_info = AccountInfo {
//             key: &Pubkey::new_unique(),
//             is_signer: false,
//             is_writable: false,
//             lamports: RefCell::new(&mut lamports).into(),
//             data: RefCell::new(data.as_mut_slice()).into(),
//             owner: &crate::ID,
//             rent_epoch: 0,
//             executable: false,
//         };
//         let mut cpi_context_account = Some(Account::try_from(account_info.as_ref()).unwrap());
//         let mut input_bytes = Vec::new();
//         inputs.serialize(&mut input_bytes).unwrap();
//         let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
//         let result = process_cpi_context(z_inputs, &mut cpi_context_account, fee_payer, &[]);
//         assert_eq!(result, Err(SystemProgramError::CpiContextMissing.into()));
//     }

//     /// Check: process cpi 3
//     #[test]
//     fn test_process_cpi_no_inputs() {
//         let fee_payer = Pubkey::new_unique();
//         let mut inputs = create_test_instruction_data(false, true, 1);
//         inputs.input_compressed_accounts_with_merkle_context = vec![];
//         inputs.output_compressed_accounts = vec![];

//         let mut lamports = 0;
//         let cpi_context_content = CpiContextAccount {
//             fee_payer: Pubkey::default(),
//             associated_merkle_tree: Pubkey::new_unique(),
//             context: vec![],
//         };
//         let mut data = vec![22, 20, 149, 218, 74, 204, 128, 166];
//         data.extend_from_slice(&cpi_context_content.try_to_vec().unwrap());
//         let account_info = AccountInfo {
//             key: &Pubkey::new_unique(),
//             is_signer: false,
//             is_writable: false,
//             lamports: RefCell::new(&mut lamports).into(),
//             data: RefCell::new(data.as_mut_slice()).into(),
//             owner: &crate::ID,
//             rent_epoch: 0,
//             executable: false,
//         };
//         let mut cpi_context_account = Some(Account::try_from(account_info.as_ref()).unwrap());
//         let mut input_bytes = Vec::new();
//         inputs.serialize(&mut input_bytes).unwrap();
//         let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
//         let result = process_cpi_context(z_inputs, &mut cpi_context_account, fee_payer, &[]);
//         assert_eq!(result, Err(SystemProgramError::NoInputs.into()));
//     }

//     /// Check: process cpi 4
//     #[test]
//     fn test_process_cpi_context_associated_tree_mismatch() {
//         let fee_payer = Pubkey::new_unique();
//         let inputs = create_test_instruction_data(true, true, 1);
//         let mut lamports = 0;
//         let merkle_tree_pubkey = Pubkey::new_unique();
//         let cpi_context_content = CpiContextAccount {
//             fee_payer: Pubkey::default(),
//             associated_merkle_tree: merkle_tree_pubkey,
//             context: vec![],
//         };
//         let mut data = vec![22, 20, 149, 218, 74, 204, 128, 166];
//         data.extend_from_slice(&cpi_context_content.try_to_vec().unwrap());
//         let account_info = AccountInfo {
//             key: &Pubkey::new_unique(),
//             is_signer: false,
//             is_writable: false,
//             lamports: RefCell::new(&mut lamports).into(),
//             data: RefCell::new(data.as_mut_slice()).into(),
//             owner: &crate::ID,
//             rent_epoch: 0,
//             executable: false,
//         };
//         let mut cpi_context_account = Some(Account::try_from(account_info.as_ref()).unwrap());
//         let mut mt_lamports = 0;
//         let mut data = vec![172, 43, 172, 186, 29, 73, 219, 84];
//         let invalid_merkle_tree_pubkey = Pubkey::new_unique();
//         let merkle_tree_account_info = AccountInfo {
//             key: &invalid_merkle_tree_pubkey,
//             is_signer: false,
//             is_writable: false,
//             lamports: RefCell::new(&mut mt_lamports).into(),
//             data: RefCell::new(data.as_mut_slice()).into(),
//             owner: &crate::ID,
//             rent_epoch: 0,
//             executable: false,
//         };
//         let remaining_accounts = &[merkle_tree_account_info];
//         let mut input_bytes = Vec::new();
//         inputs.serialize(&mut input_bytes).unwrap();
//         let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
//         let result = process_cpi_context(
//             z_inputs,
//             &mut cpi_context_account,
//             fee_payer,
//             remaining_accounts,
//         );
//         assert_eq!(
//             result,
//             Err(SystemProgramError::CpiContextAssociatedMerkleTreeMismatch.into())
//         );
//     }

//     /// Check: process cpi 5
//     #[test]
//     fn test_process_cpi_context_no_set_context() {
//         let fee_payer = Pubkey::new_unique();
//         let inputs = create_test_instruction_data(false, false, 1);
//         let mut lamports = 0;
//         let merkle_tree_pubkey = Pubkey::new_unique();
//         let cpi_context_content = CpiContextAccount {
//             fee_payer: Pubkey::default(),
//             associated_merkle_tree: merkle_tree_pubkey,
//             context: vec![],
//         };
//         let mut data = vec![22, 20, 149, 218, 74, 204, 128, 166];
//         data.extend_from_slice(&cpi_context_content.try_to_vec().unwrap());
//         let account_info = AccountInfo {
//             key: &Pubkey::new_unique(),
//             is_signer: false,
//             is_writable: false,
//             lamports: RefCell::new(&mut lamports).into(),
//             data: RefCell::new(data.as_mut_slice()).into(),
//             owner: &crate::ID,
//             rent_epoch: 0,
//             executable: false,
//         };
//         let mut cpi_context_account = Some(Account::try_from(account_info.as_ref()).unwrap());
//         let mut mt_lamports = 0;
//         let mut data = vec![172, 43, 172, 186, 29, 73, 219, 84];
//         let merkle_tree_account_info = AccountInfo {
//             key: &merkle_tree_pubkey,
//             is_signer: false,
//             is_writable: false,
//             lamports: RefCell::new(&mut mt_lamports).into(),
//             data: RefCell::new(data.as_mut_slice()).into(),
//             owner: &crate::ID,
//             rent_epoch: 0,
//             executable: false,
//         };
//         let remaining_accounts = &[merkle_tree_account_info];
//         let mut input_bytes = Vec::new();
//         inputs.serialize(&mut input_bytes).unwrap();
//         let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
//         let result = process_cpi_context(
//             z_inputs,
//             &mut cpi_context_account,
//             fee_payer,
//             remaining_accounts,
//         );
//         assert_eq!(result, Err(SystemProgramError::CpiContextEmpty.into()));
//     }

//     /// Check: process cpi 6
//     #[test]
//     fn test_process_cpi_context_empty_context_error() {
//         let fee_payer = Pubkey::default();
//         let inputs = create_test_instruction_data(false, true, 1);
//         let mut lamports = 0;
//         let merkle_tree_pubkey = Pubkey::new_unique();
//         let cpi_context_content = CpiContextAccount {
//             fee_payer: Pubkey::default(),
//             associated_merkle_tree: merkle_tree_pubkey,
//             context: vec![],
//         };
//         let mut data = vec![22, 20, 149, 218, 74, 204, 128, 166];
//         data.extend_from_slice(&cpi_context_content.try_to_vec().unwrap());
//         let account_info = AccountInfo {
//             key: &Pubkey::new_unique(),
//             is_signer: false,
//             is_writable: false,
//             lamports: RefCell::new(&mut lamports).into(),
//             data: RefCell::new(data.as_mut_slice()).into(),
//             owner: &crate::ID,
//             rent_epoch: 0,
//             executable: false,
//         };
//         let mut cpi_context_account = Some(Account::try_from(account_info.as_ref()).unwrap());
//         let mut mt_lamports = 0;
//         let mut data = vec![172, 43, 172, 186, 29, 73, 219, 84];
//         let merkle_tree_account_info = AccountInfo {
//             key: &merkle_tree_pubkey,
//             is_signer: false,
//             is_writable: false,
//             lamports: RefCell::new(&mut mt_lamports).into(),
//             data: RefCell::new(data.as_mut_slice()).into(),
//             owner: &crate::ID,
//             rent_epoch: 0,
//             executable: false,
//         };
//         let remaining_accounts = &[merkle_tree_account_info];
//         let mut input_bytes = Vec::new();
//         inputs.serialize(&mut input_bytes).unwrap();
//         let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
//         let result = process_cpi_context(
//             z_inputs,
//             &mut cpi_context_account,
//             fee_payer,
//             remaining_accounts,
//         );
//         assert_eq!(
//             result,
//             Err(SystemProgramError::CpiContextFeePayerMismatch.into())
//         );
//     }

//     /// Check: process cpi 6
//     #[test]
//     fn test_process_cpi_context_fee_payer_mismatch_error() {
//         let fee_payer = Pubkey::new_unique();
//         let inputs = create_test_instruction_data(true, true, 1);
//         let mut lamports = 0;
//         let merkle_tree_pubkey = Pubkey::new_unique();
//         let cpi_context_content = CpiContextAccount {
//             fee_payer: Pubkey::default(),
//             associated_merkle_tree: merkle_tree_pubkey,
//             context: vec![],
//         };
//         let mut data = vec![22, 20, 149, 218, 74, 204, 128, 166];
//         data.extend_from_slice(&cpi_context_content.try_to_vec().unwrap());
//         let account_info = AccountInfo {
//             key: &Pubkey::new_unique(),
//             is_signer: false,
//             is_writable: false,
//             lamports: RefCell::new(&mut lamports).into(),
//             data: RefCell::new(data.as_mut_slice()).into(),
//             owner: &crate::ID,
//             rent_epoch: 0,
//             executable: false,
//         };
//         let mut cpi_context_account = Some(Account::try_from(account_info.as_ref()).unwrap());
//         let mut mt_lamports = 0;
//         let mut data = vec![172, 43, 172, 186, 29, 73, 219, 84];
//         let merkle_tree_account_info = AccountInfo {
//             key: &merkle_tree_pubkey,
//             is_signer: false,
//             is_writable: false,
//             lamports: RefCell::new(&mut mt_lamports).into(),
//             data: RefCell::new(data.as_mut_slice()).into(),
//             owner: &crate::ID,
//             rent_epoch: 0,
//             executable: false,
//         };
//         let remaining_accounts = &[merkle_tree_account_info];
//         let mut input_bytes = Vec::new();
//         inputs.serialize(&mut input_bytes).unwrap();
//         let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
//         let result = process_cpi_context(
//             z_inputs,
//             &mut cpi_context_account,
//             fee_payer,
//             remaining_accounts,
//         );
//         assert!(result.is_ok());
//         let invalid_fee_payer = Pubkey::new_unique();
//         let inputs = create_test_instruction_data(false, true, 1);
//         let mut input_bytes = Vec::new();
//         inputs.serialize(&mut input_bytes).unwrap();
//         let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
//         let result = process_cpi_context(
//             z_inputs,
//             &mut cpi_context_account,
//             invalid_fee_payer,
//             remaining_accounts,
//         );
//         assert_eq!(
//             result,
//             Err(SystemProgramError::CpiContextFeePayerMismatch.into())
//         );
//     }

//     #[test]
//     fn test_process_cpi_context_set_context() {
//         let fee_payer = Pubkey::new_unique();
//         let mut inputs = create_test_instruction_data(true, true, 1);
//         let mut lamports = 0;
//         let merkle_tree_pubkey = Pubkey::new_unique();
//         let cpi_context_content = CpiContextAccount {
//             fee_payer: Pubkey::default(),
//             associated_merkle_tree: merkle_tree_pubkey,
//             context: vec![],
//         };
//         let mut data = vec![22, 20, 149, 218, 74, 204, 128, 166];
//         data.extend_from_slice(&cpi_context_content.try_to_vec().unwrap());
//         let account_info = AccountInfo {
//             key: &Pubkey::new_unique(),
//             is_signer: false,
//             is_writable: false,
//             lamports: RefCell::new(&mut lamports).into(),
//             data: RefCell::new(data.as_mut_slice()).into(),
//             owner: &crate::ID,
//             rent_epoch: 0,
//             executable: false,
//         };
//         let mut cpi_context_account = Some(Account::try_from(account_info.as_ref()).unwrap());
//         let mut mt_lamports = 0;
//         let mut data = vec![172, 43, 172, 186, 29, 73, 219, 84];
//         let merkle_tree_account_info = AccountInfo {
//             key: &merkle_tree_pubkey,
//             is_signer: false,
//             is_writable: false,
//             lamports: RefCell::new(&mut mt_lamports).into(),
//             data: RefCell::new(data.as_mut_slice()).into(),
//             owner: &crate::ID,
//             rent_epoch: 0,
//             executable: false,
//         };
//         let remaining_accounts = &[merkle_tree_account_info];
//         let mut input_bytes = Vec::new();
//         inputs.serialize(&mut input_bytes).unwrap();
//         let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
//         let result = process_cpi_context(
//             z_inputs,
//             &mut cpi_context_account,
//             fee_payer,
//             remaining_accounts,
//         );
//         assert!(result.is_ok());
//         assert_eq!(cpi_context_account.as_ref().unwrap().context.len(), 1);
//         assert_eq!(cpi_context_account.as_ref().unwrap().fee_payer, fee_payer);
//         clean_input_data(&mut inputs);
//         assert_eq!(cpi_context_account.as_ref().unwrap().context[0], inputs);
//         assert_eq!(result.unwrap(), None);
//     }

//     #[test]
//     fn test_process_cpi_context_combine() {
//         let fee_payer = Pubkey::new_unique();
//         let mut inputs = create_test_instruction_data(true, true, 1);
//         let malicious_inputs = create_test_instruction_data(true, true, 100);

//         let mut lamports = 0;
//         let merkle_tree_pubkey = Pubkey::new_unique();
//         let cpi_context_content = CpiContextAccount {
//             fee_payer: Pubkey::default(),
//             associated_merkle_tree: merkle_tree_pubkey,
//             context: vec![malicious_inputs],
//         };
//         let mut data = vec![22, 20, 149, 218, 74, 204, 128, 166];
//         data.extend_from_slice(&cpi_context_content.try_to_vec().unwrap());
//         let account_info = AccountInfo {
//             key: &Pubkey::new_unique(),
//             is_signer: false,
//             is_writable: false,
//             lamports: RefCell::new(&mut lamports).into(),
//             data: RefCell::new(data.as_mut_slice()).into(),
//             owner: &crate::ID,
//             rent_epoch: 0,
//             executable: false,
//         };
//         let mut cpi_context_account = Some(Account::try_from(account_info.as_ref()).unwrap());
//         let mut mt_lamports = 0;
//         let mut data = vec![172, 43, 172, 186, 29, 73, 219, 84];
//         let merkle_tree_account_info = AccountInfo {
//             key: &merkle_tree_pubkey,
//             is_signer: false,
//             is_writable: false,
//             lamports: RefCell::new(&mut mt_lamports).into(),
//             data: RefCell::new(data.as_mut_slice()).into(),
//             owner: &crate::ID,
//             rent_epoch: 0,
//             executable: false,
//         };
//         let remaining_accounts = &[merkle_tree_account_info];
//         let mut input_bytes = Vec::new();
//         inputs.serialize(&mut input_bytes).unwrap();
//         let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
//         let result = process_cpi_context(
//             z_inputs,
//             &mut cpi_context_account,
//             fee_payer,
//             remaining_accounts,
//         );
//         assert!(result.is_ok());
//         assert_eq!(cpi_context_account.as_ref().unwrap().context.len(), 1);
//         assert_eq!(cpi_context_account.as_ref().unwrap().fee_payer, fee_payer);
//         clean_input_data(&mut inputs);

//         assert_eq!(cpi_context_account.as_ref().unwrap().context[0], inputs);
//         assert_eq!(result.unwrap(), None);
//         for i in 2..10 {
//             let pre_account_info = account_info.data.clone();
//             let mut inputs = create_test_instruction_data(false, true, i);
//             let mut input_bytes = Vec::new();
//             inputs.serialize(&mut input_bytes).unwrap();
//             let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
//             let result = process_cpi_context(
//                 z_inputs,
//                 &mut cpi_context_account,
//                 fee_payer,
//                 remaining_accounts,
//             );
//             assert!(result.is_ok());
//             assert_eq!(
//                 cpi_context_account.as_ref().unwrap().context.len(),
//                 i as usize
//             );
//             assert_eq!(cpi_context_account.as_ref().unwrap().fee_payer, fee_payer);
//             clean_input_data(&mut inputs);
//             assert_eq!(
//                 cpi_context_account.as_ref().unwrap().context[(i - 1) as usize],
//                 inputs
//             );
//             assert_eq!(result.unwrap(), None);
//             assert_eq!(account_info.data, pre_account_info);
//         }
//         // account info data doesn't change hence we change it manually for the test here
//         let mut data = vec![22, 20, 149, 218, 74, 204, 128, 166];
//         let mut struct_data = Vec::new();
//         cpi_context_account
//             .as_ref()
//             .unwrap()
//             .serialize(&mut struct_data)
//             .unwrap();
//         data.extend_from_slice(struct_data.as_slice());
//         let mut lamports = 0;

//         let account_info = AccountInfo {
//             key: &Pubkey::new_unique(),
//             is_signer: false,
//             is_writable: false,
//             lamports: RefCell::new(&mut lamports).into(),
//             data: RefCell::new(data.as_mut_slice()).into(),
//             owner: &crate::ID,
//             rent_epoch: 0,
//             executable: false,
//         };
//         let mut cpi_context_account =
//             Some(Account::<CpiContextAccount>::try_from(account_info.as_ref()).unwrap());

//         let inputs = create_test_instruction_data(false, false, 10);
//         let mut input_bytes = Vec::new();
//         inputs.serialize(&mut input_bytes).unwrap();
//         let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();

//         let result = process_cpi_context(
//             z_inputs,
//             &mut cpi_context_account,
//             fee_payer,
//             remaining_accounts,
//         );
//         assert!(result.is_ok());
//         let result = result.unwrap().unwrap();

//         assert!(result.new_address_params.is_empty());
//         for i in 1..10 {
//             assert_eq!(
//                 result.output_compressed_accounts[i]
//                     .compressed_account
//                     .lamports,
//                 i as u64
//             );
//             assert_eq!(
//                 result.input_compressed_accounts_with_merkle_context[i]
//                     .compressed_account
//                     .lamports,
//                 i as u64
//             );
//         }
//         assert_eq!(
//             cpi_context_account.as_ref().unwrap().associated_merkle_tree,
//             merkle_tree_pubkey
//         );
//         assert_eq!(
//             cpi_context_account.as_ref().unwrap().fee_payer,
//             Pubkey::default()
//         );
//         assert_eq!(cpi_context_account.as_ref().unwrap().context.len(), 0);
//     }
// }
