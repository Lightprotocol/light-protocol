use crate::Result;
use light_compressed_account::instruction_data::zero_copy::ZInstructionDataInvokeCpi;
use pinocchio::{account_info::AccountInfo, msg, pubkey::Pubkey};

use super::account::{deserialize_cpi_context_account, ZCpiContextAccount};
use crate::errors::SystemProgramError;

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
pub fn process_cpi_context<'a, 'info>(
    mut inputs: ZInstructionDataInvokeCpi<'a>,
    cpi_context_account: &mut Option<&'info AccountInfo>,
    fee_payer: Pubkey,
    remaining_accounts: &[AccountInfo],
) -> Result<Option<(ZInstructionDataInvokeCpi<'a>, usize)>> {
    let cpi_context = &inputs.cpi_context;
    if cpi_context_account.is_some() && cpi_context.is_none() {
        msg!("cpi context account is some but cpi context is none");
        return Err(SystemProgramError::CpiContextMissing.into());
    }
    let mut num_cpi_contexts = 0;
    if let Some(cpi_context) = cpi_context {
        let cpi_context_account = match cpi_context_account {
            Some(cpi_context_account) => cpi_context_account,
            None => return Err(SystemProgramError::CpiContextAccountUndefined.into()),
        };
        let mut cpi_context_account = deserialize_cpi_context_account(cpi_context_account)?;
        let index = if !inputs
            .input_compressed_accounts_with_merkle_context
            .is_empty()
        {
            inputs.input_compressed_accounts_with_merkle_context[0]
                .merkle_context
                .merkle_tree_pubkey_index
        } else if !inputs.output_compressed_accounts.is_empty() {
            inputs.output_compressed_accounts[0].merkle_tree_index
        } else {
            return Err(SystemProgramError::NoInputs.into());
        };
        let first_merkle_tree_pubkey = remaining_accounts[index as usize].key();
        if *cpi_context_account.associated_merkle_tree != first_merkle_tree_pubkey.into() {
            msg!(
                "first_merkle_tree_pubkey {:?} != associated_merkle_tree {:?}",
                first_merkle_tree_pubkey,
                cpi_context_account.associated_merkle_tree
            );
            return Err(SystemProgramError::CpiContextAssociatedMerkleTreeMismatch.into());
        }
        if cpi_context.set_context() {
            set_cpi_context(fee_payer, &mut cpi_context_account, inputs)?;
            return Ok(None);
        } else {
            if cpi_context_account.context.is_empty() {
                msg!("cpi context account : {:?}", cpi_context_account);
                msg!("fee payer : {:?}", fee_payer);
                msg!("cpi context  : {:?}", cpi_context);
                return Err(SystemProgramError::CpiContextEmpty.into());
            } else if *cpi_context_account.fee_payer != fee_payer.into()
                || cpi_context.first_set_context()
            {
                msg!("cpi context account : {:?}", cpi_context_account);
                msg!("fee payer : {:?}", fee_payer);
                msg!("cpi context  : {:?}", cpi_context);
                return Err(SystemProgramError::CpiContextFeePayerMismatch.into());
            }

            num_cpi_contexts = cpi_context_account.context.len();
            inputs.combine(cpi_context_account.context);
            // Reset cpi context account
            cpi_context_account.context = Vec::new();
            *cpi_context_account.fee_payer = Pubkey::default().into();
        }
    }
    Ok(Some((inputs, num_cpi_contexts)))
}

pub fn set_cpi_context<'a>(
    fee_payer: Pubkey,
    cpi_context_account: &mut ZCpiContextAccount<'a>,
    inputs: ZInstructionDataInvokeCpi<'a>,
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
    if inputs.cpi_context.unwrap().first_set_context() {
        if !inputs.new_address_params.is_empty() {
            unimplemented!("new addresses are not supported with cpi context");
        }
        cpi_context_account.context = vec![inputs];
        *cpi_context_account.fee_payer = fee_payer.into();
    } else if *cpi_context_account.fee_payer == fee_payer.into()
        && !cpi_context_account.context.is_empty()
    {
        if !inputs.new_address_params.is_empty() {
            unimplemented!("new addresses are not supported with cpi context");
        }
        cpi_context_account.context.push(inputs);
    } else {
        msg!(format!(" {:?} != {:?}", fee_payer, cpi_context_account.fee_payer).as_str());
        return Err(SystemProgramError::CpiContextFeePayerMismatch.into());
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
    use std::cell::RefCell;

    use light_compressed_account::{
        compressed_account::{
            CompressedAccount, PackedCompressedAccountWithMerkleContext, PackedMerkleContext,
        },
        instruction_data::{
            cpi_context::CompressedCpiContext, data::OutputCompressedAccountWithPackedContext,
            invoke_cpi::InstructionDataInvokeCpi,
        },
    };
    use light_zero_copy::borsh::Deserialize;
    use pinocchio::pubkey::Pubkey;

    use super::*;

    fn clean_input_data(inputs: &mut InstructionDataInvokeCpi) {
        inputs.cpi_context = None;
        inputs.compress_or_decompress_lamports = None;
        inputs.relay_fee = None;
        inputs.proof = None;
    }

    fn create_test_cpi_context_account() -> CpiContextAccount {
        CpiContextAccount {
            fee_payer: Pubkey::new_unique(),
            associated_merkle_tree: Pubkey::new_unique(),
            context: vec![],
        }
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
                        owner: Pubkey::new_unique(),
                        lamports: iter.into(),
                        address: None,
                        data: None,
                    },
                    merkle_context: PackedMerkleContext {
                        merkle_tree_pubkey_index: 0,
                        nullifier_queue_pubkey_index: iter,
                        leaf_index: 0,
                        prove_by_index: false,
                    },
                    root_index: iter.into(),
                    read_only: false,
                },
            ],
            output_compressed_accounts: vec![OutputCompressedAccountWithPackedContext {
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_unique(),
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

    #[test]
    fn test_set_cpi_context_first_invocation() {
        let fee_payer = Pubkey::new_unique();
        let mut cpi_context_account = create_test_cpi_context_account();
        let mut inputs = create_test_instruction_data(true, true, 1);
        let mut input_bytes = Vec::new();
        inputs.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();

        let result = set_cpi_context(fee_payer, &mut cpi_context_account, z_inputs);
        assert!(result.is_ok());
        assert_eq!(cpi_context_account.fee_payer, fee_payer);
        assert_eq!(cpi_context_account.context.len(), 1);
        assert_ne!(cpi_context_account.context[0], inputs);
        clean_input_data(&mut inputs);
        assert_eq!(cpi_context_account.context[0], inputs);
    }

    #[test]
    fn test_set_cpi_context_subsequent_invocation() {
        let fee_payer = Pubkey::new_unique();
        let mut cpi_context_account = create_test_cpi_context_account();
        let inputs_first = create_test_instruction_data(true, true, 1);
        let mut input_bytes = Vec::new();
        inputs_first.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();

        set_cpi_context(fee_payer, &mut cpi_context_account, z_inputs).unwrap();

        let mut inputs_subsequent = create_test_instruction_data(false, true, 2);
        let mut input_bytes = Vec::new();
        inputs_subsequent.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let result = set_cpi_context(fee_payer, &mut cpi_context_account, z_inputs);
        assert!(result.is_ok());
        assert_eq!(cpi_context_account.context.len(), 2);
        clean_input_data(&mut inputs_subsequent);
        assert_eq!(cpi_context_account.context[1], inputs_subsequent);
    }

    #[test]
    fn test_set_cpi_context_fee_payer_mismatch() {
        let fee_payer = Pubkey::new_unique();
        let mut cpi_context_account = create_test_cpi_context_account();
        let inputs_first = create_test_instruction_data(true, true, 1);
        let mut input_bytes = Vec::new();
        inputs_first.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        set_cpi_context(fee_payer, &mut cpi_context_account, z_inputs).unwrap();

        let different_fee_payer = Pubkey::new_unique();
        let inputs_subsequent = create_test_instruction_data(false, true, 2);
        let mut input_bytes = Vec::new();
        inputs_subsequent.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let result = set_cpi_context(different_fee_payer, &mut cpi_context_account, z_inputs);
        assert_eq!(
            result.unwrap_err(),
            SystemProgramError::CpiContextFeePayerMismatch.into()
        );
    }

    #[test]
    fn test_set_cpi_context_without_first_context() {
        let fee_payer = Pubkey::new_unique();
        let mut cpi_context_account = create_test_cpi_context_account();
        let inputs_first = create_test_instruction_data(false, true, 1);
        let mut input_bytes = Vec::new();
        inputs_first.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let result = set_cpi_context(fee_payer, &mut cpi_context_account, z_inputs);
        assert_eq!(
            result,
            Err(SystemProgramError::CpiContextFeePayerMismatch.into())
        );
    }

    /// Check: process cpi 1
    #[test]
    fn test_process_cpi_context_both_none() {
        let fee_payer = Pubkey::new_unique();
        let inputs = create_test_instruction_data(false, true, 1);
        let mut cpi_context_account: Option<Account<CpiContextAccount>> = None;
        let mut input_bytes = Vec::new();
        inputs.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let result = process_cpi_context(z_inputs, &mut cpi_context_account, fee_payer, &[]);
        assert_eq!(
            result,
            Err(SystemProgramError::CpiContextAccountUndefined.into())
        );
    }

    /// Check: process cpi 1
    #[test]
    fn test_process_cpi_context_account_none_context_some() {
        let fee_payer = Pubkey::new_unique();
        let inputs = create_test_instruction_data(false, true, 1);
        let mut cpi_context_account: Option<Account<CpiContextAccount>> = None;
        let mut input_bytes = Vec::new();
        inputs.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let result = process_cpi_context(z_inputs, &mut cpi_context_account, fee_payer, &[]);
        assert_eq!(
            result,
            Err(SystemProgramError::CpiContextAccountUndefined.into())
        );
    }

    /// Check: process cpi 2
    #[test]
    fn test_process_cpi_context_account_some_context_none() {
        let fee_payer = Pubkey::new_unique();
        let inputs = InstructionDataInvokeCpi {
            cpi_context: None,
            ..create_test_instruction_data(false, true, 1)
        };
        let mut lamports = 0;
        let cpi_context_content = CpiContextAccount {
            fee_payer: Pubkey::default(),
            associated_merkle_tree: Pubkey::new_unique(),
            context: vec![],
        };
        let mut data = vec![22, 20, 149, 218, 74, 204, 128, 166];
        data.extend_from_slice(&cpi_context_content.try_to_vec().unwrap());
        let account_info = AccountInfo {
            key: &Pubkey::new_unique(),
            is_signer: false,
            is_writable: false,
            lamports: RefCell::new(&mut lamports).into(),
            data: RefCell::new(data.as_mut_slice()).into(),
            owner: &crate::ID,
            rent_epoch: 0,
            executable: false,
        };
        let mut cpi_context_account = Some(Account::try_from(account_info.as_ref()).unwrap());
        let mut input_bytes = Vec::new();
        inputs.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let result = process_cpi_context(z_inputs, &mut cpi_context_account, fee_payer, &[]);
        assert_eq!(result, Err(SystemProgramError::CpiContextMissing.into()));
    }

    /// Check: process cpi 3
    #[test]
    fn test_process_cpi_no_inputs() {
        let fee_payer = Pubkey::new_unique();
        let mut inputs = create_test_instruction_data(false, true, 1);
        inputs.input_compressed_accounts_with_merkle_context = vec![];
        inputs.output_compressed_accounts = vec![];

        let mut lamports = 0;
        let cpi_context_content = CpiContextAccount {
            fee_payer: Pubkey::default(),
            associated_merkle_tree: Pubkey::new_unique(),
            context: vec![],
        };
        let mut data = vec![22, 20, 149, 218, 74, 204, 128, 166];
        data.extend_from_slice(&cpi_context_content.try_to_vec().unwrap());
        let account_info = AccountInfo {
            key: &Pubkey::new_unique(),
            is_signer: false,
            is_writable: false,
            lamports: RefCell::new(&mut lamports).into(),
            data: RefCell::new(data.as_mut_slice()).into(),
            owner: &crate::ID,
            rent_epoch: 0,
            executable: false,
        };
        let mut cpi_context_account = Some(Account::try_from(account_info.as_ref()).unwrap());
        let mut input_bytes = Vec::new();
        inputs.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let result = process_cpi_context(z_inputs, &mut cpi_context_account, fee_payer, &[]);
        assert_eq!(result, Err(SystemProgramError::NoInputs.into()));
    }

    /// Check: process cpi 4
    #[test]
    fn test_process_cpi_context_associated_tree_mismatch() {
        let fee_payer = Pubkey::new_unique();
        let inputs = create_test_instruction_data(true, true, 1);
        let mut lamports = 0;
        let merkle_tree_pubkey = Pubkey::new_unique();
        let cpi_context_content = CpiContextAccount {
            fee_payer: Pubkey::default(),
            associated_merkle_tree: merkle_tree_pubkey,
            context: vec![],
        };
        let mut data = vec![22, 20, 149, 218, 74, 204, 128, 166];
        data.extend_from_slice(&cpi_context_content.try_to_vec().unwrap());
        let account_info = AccountInfo {
            key: &Pubkey::new_unique(),
            is_signer: false,
            is_writable: false,
            lamports: RefCell::new(&mut lamports).into(),
            data: RefCell::new(data.as_mut_slice()).into(),
            owner: &crate::ID,
            rent_epoch: 0,
            executable: false,
        };
        let mut cpi_context_account = Some(Account::try_from(account_info.as_ref()).unwrap());
        let mut mt_lamports = 0;
        let mut data = vec![172, 43, 172, 186, 29, 73, 219, 84];
        let invalid_merkle_tree_pubkey = Pubkey::new_unique();
        let merkle_tree_account_info = AccountInfo {
            key: &invalid_merkle_tree_pubkey,
            is_signer: false,
            is_writable: false,
            lamports: RefCell::new(&mut mt_lamports).into(),
            data: RefCell::new(data.as_mut_slice()).into(),
            owner: &crate::ID,
            rent_epoch: 0,
            executable: false,
        };
        let remaining_accounts = &[merkle_tree_account_info];
        let mut input_bytes = Vec::new();
        inputs.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let result = process_cpi_context(
            z_inputs,
            &mut cpi_context_account,
            fee_payer,
            remaining_accounts,
        );
        assert_eq!(
            result,
            Err(SystemProgramError::CpiContextAssociatedMerkleTreeMismatch.into())
        );
    }

    /// Check: process cpi 5
    #[test]
    fn test_process_cpi_context_no_set_context() {
        let fee_payer = Pubkey::new_unique();
        let inputs = create_test_instruction_data(false, false, 1);
        let mut lamports = 0;
        let merkle_tree_pubkey = Pubkey::new_unique();
        let cpi_context_content = CpiContextAccount {
            fee_payer: Pubkey::default(),
            associated_merkle_tree: merkle_tree_pubkey,
            context: vec![],
        };
        let mut data = vec![22, 20, 149, 218, 74, 204, 128, 166];
        data.extend_from_slice(&cpi_context_content.try_to_vec().unwrap());
        let account_info = AccountInfo {
            key: &Pubkey::new_unique(),
            is_signer: false,
            is_writable: false,
            lamports: RefCell::new(&mut lamports).into(),
            data: RefCell::new(data.as_mut_slice()).into(),
            owner: &crate::ID,
            rent_epoch: 0,
            executable: false,
        };
        let mut cpi_context_account = Some(Account::try_from(account_info.as_ref()).unwrap());
        let mut mt_lamports = 0;
        let mut data = vec![172, 43, 172, 186, 29, 73, 219, 84];
        let merkle_tree_account_info = AccountInfo {
            key: &merkle_tree_pubkey,
            is_signer: false,
            is_writable: false,
            lamports: RefCell::new(&mut mt_lamports).into(),
            data: RefCell::new(data.as_mut_slice()).into(),
            owner: &crate::ID,
            rent_epoch: 0,
            executable: false,
        };
        let remaining_accounts = &[merkle_tree_account_info];
        let mut input_bytes = Vec::new();
        inputs.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let result = process_cpi_context(
            z_inputs,
            &mut cpi_context_account,
            fee_payer,
            remaining_accounts,
        );
        assert_eq!(result, Err(SystemProgramError::CpiContextEmpty.into()));
    }

    /// Check: process cpi 6
    #[test]
    fn test_process_cpi_context_empty_context_error() {
        let fee_payer = Pubkey::default();
        let inputs = create_test_instruction_data(false, true, 1);
        let mut lamports = 0;
        let merkle_tree_pubkey = Pubkey::new_unique();
        let cpi_context_content = CpiContextAccount {
            fee_payer: Pubkey::default(),
            associated_merkle_tree: merkle_tree_pubkey,
            context: vec![],
        };
        let mut data = vec![22, 20, 149, 218, 74, 204, 128, 166];
        data.extend_from_slice(&cpi_context_content.try_to_vec().unwrap());
        let account_info = AccountInfo {
            key: &Pubkey::new_unique(),
            is_signer: false,
            is_writable: false,
            lamports: RefCell::new(&mut lamports).into(),
            data: RefCell::new(data.as_mut_slice()).into(),
            owner: &crate::ID,
            rent_epoch: 0,
            executable: false,
        };
        let mut cpi_context_account = Some(Account::try_from(account_info.as_ref()).unwrap());
        let mut mt_lamports = 0;
        let mut data = vec![172, 43, 172, 186, 29, 73, 219, 84];
        let merkle_tree_account_info = AccountInfo {
            key: &merkle_tree_pubkey,
            is_signer: false,
            is_writable: false,
            lamports: RefCell::new(&mut mt_lamports).into(),
            data: RefCell::new(data.as_mut_slice()).into(),
            owner: &crate::ID,
            rent_epoch: 0,
            executable: false,
        };
        let remaining_accounts = &[merkle_tree_account_info];
        let mut input_bytes = Vec::new();
        inputs.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let result = process_cpi_context(
            z_inputs,
            &mut cpi_context_account,
            fee_payer,
            remaining_accounts,
        );
        assert_eq!(
            result,
            Err(SystemProgramError::CpiContextFeePayerMismatch.into())
        );
    }

    /// Check: process cpi 6
    #[test]
    fn test_process_cpi_context_fee_payer_mismatch_error() {
        let fee_payer = Pubkey::new_unique();
        let inputs = create_test_instruction_data(true, true, 1);
        let mut lamports = 0;
        let merkle_tree_pubkey = Pubkey::new_unique();
        let cpi_context_content = CpiContextAccount {
            fee_payer: Pubkey::default(),
            associated_merkle_tree: merkle_tree_pubkey,
            context: vec![],
        };
        let mut data = vec![22, 20, 149, 218, 74, 204, 128, 166];
        data.extend_from_slice(&cpi_context_content.try_to_vec().unwrap());
        let account_info = AccountInfo {
            key: &Pubkey::new_unique(),
            is_signer: false,
            is_writable: false,
            lamports: RefCell::new(&mut lamports).into(),
            data: RefCell::new(data.as_mut_slice()).into(),
            owner: &crate::ID,
            rent_epoch: 0,
            executable: false,
        };
        let mut cpi_context_account = Some(Account::try_from(account_info.as_ref()).unwrap());
        let mut mt_lamports = 0;
        let mut data = vec![172, 43, 172, 186, 29, 73, 219, 84];
        let merkle_tree_account_info = AccountInfo {
            key: &merkle_tree_pubkey,
            is_signer: false,
            is_writable: false,
            lamports: RefCell::new(&mut mt_lamports).into(),
            data: RefCell::new(data.as_mut_slice()).into(),
            owner: &crate::ID,
            rent_epoch: 0,
            executable: false,
        };
        let remaining_accounts = &[merkle_tree_account_info];
        let mut input_bytes = Vec::new();
        inputs.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let result = process_cpi_context(
            z_inputs,
            &mut cpi_context_account,
            fee_payer,
            remaining_accounts,
        );
        assert!(result.is_ok());
        let invalid_fee_payer = Pubkey::new_unique();
        let inputs = create_test_instruction_data(false, true, 1);
        let mut input_bytes = Vec::new();
        inputs.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let result = process_cpi_context(
            z_inputs,
            &mut cpi_context_account,
            invalid_fee_payer,
            remaining_accounts,
        );
        assert_eq!(
            result,
            Err(SystemProgramError::CpiContextFeePayerMismatch.into())
        );
    }

    #[test]
    fn test_process_cpi_context_set_context() {
        let fee_payer = Pubkey::new_unique();
        let mut inputs = create_test_instruction_data(true, true, 1);
        let mut lamports = 0;
        let merkle_tree_pubkey = Pubkey::new_unique();
        let cpi_context_content = CpiContextAccount {
            fee_payer: Pubkey::default(),
            associated_merkle_tree: merkle_tree_pubkey,
            context: vec![],
        };
        let mut data = vec![22, 20, 149, 218, 74, 204, 128, 166];
        data.extend_from_slice(&cpi_context_content.try_to_vec().unwrap());
        let account_info = AccountInfo {
            key: &Pubkey::new_unique(),
            is_signer: false,
            is_writable: false,
            lamports: RefCell::new(&mut lamports).into(),
            data: RefCell::new(data.as_mut_slice()).into(),
            owner: &crate::ID,
            rent_epoch: 0,
            executable: false,
        };
        let mut cpi_context_account = Some(Account::try_from(account_info.as_ref()).unwrap());
        let mut mt_lamports = 0;
        let mut data = vec![172, 43, 172, 186, 29, 73, 219, 84];
        let merkle_tree_account_info = AccountInfo {
            key: &merkle_tree_pubkey,
            is_signer: false,
            is_writable: false,
            lamports: RefCell::new(&mut mt_lamports).into(),
            data: RefCell::new(data.as_mut_slice()).into(),
            owner: &crate::ID,
            rent_epoch: 0,
            executable: false,
        };
        let remaining_accounts = &[merkle_tree_account_info];
        let mut input_bytes = Vec::new();
        inputs.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let result = process_cpi_context(
            z_inputs,
            &mut cpi_context_account,
            fee_payer,
            remaining_accounts,
        );
        assert!(result.is_ok());
        assert_eq!(cpi_context_account.as_ref().unwrap().context.len(), 1);
        assert_eq!(cpi_context_account.as_ref().unwrap().fee_payer, fee_payer);
        clean_input_data(&mut inputs);
        assert_eq!(cpi_context_account.as_ref().unwrap().context[0], inputs);
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_process_cpi_context_combine() {
        let fee_payer = Pubkey::new_unique();
        let mut inputs = create_test_instruction_data(true, true, 1);
        let malicious_inputs = create_test_instruction_data(true, true, 100);

        let mut lamports = 0;
        let merkle_tree_pubkey = Pubkey::new_unique();
        let cpi_context_content = CpiContextAccount {
            fee_payer: Pubkey::default(),
            associated_merkle_tree: merkle_tree_pubkey,
            context: vec![malicious_inputs],
        };
        let mut data = vec![22, 20, 149, 218, 74, 204, 128, 166];
        data.extend_from_slice(&cpi_context_content.try_to_vec().unwrap());
        let account_info = AccountInfo {
            key: &Pubkey::new_unique(),
            is_signer: false,
            is_writable: false,
            lamports: RefCell::new(&mut lamports).into(),
            data: RefCell::new(data.as_mut_slice()).into(),
            owner: &crate::ID,
            rent_epoch: 0,
            executable: false,
        };
        let mut cpi_context_account = Some(Account::try_from(account_info.as_ref()).unwrap());
        let mut mt_lamports = 0;
        let mut data = vec![172, 43, 172, 186, 29, 73, 219, 84];
        let merkle_tree_account_info = AccountInfo {
            key: &merkle_tree_pubkey,
            is_signer: false,
            is_writable: false,
            lamports: RefCell::new(&mut mt_lamports).into(),
            data: RefCell::new(data.as_mut_slice()).into(),
            owner: &crate::ID,
            rent_epoch: 0,
            executable: false,
        };
        let remaining_accounts = &[merkle_tree_account_info];
        let mut input_bytes = Vec::new();
        inputs.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
        let result = process_cpi_context(
            z_inputs,
            &mut cpi_context_account,
            fee_payer,
            remaining_accounts,
        );
        assert!(result.is_ok());
        assert_eq!(cpi_context_account.as_ref().unwrap().context.len(), 1);
        assert_eq!(cpi_context_account.as_ref().unwrap().fee_payer, fee_payer);
        clean_input_data(&mut inputs);

        assert_eq!(cpi_context_account.as_ref().unwrap().context[0], inputs);
        assert_eq!(result.unwrap(), None);
        for i in 2..10 {
            let pre_account_info = account_info.data.clone();
            let mut inputs = create_test_instruction_data(false, true, i);
            let mut input_bytes = Vec::new();
            inputs.serialize(&mut input_bytes).unwrap();
            let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
            let result = process_cpi_context(
                z_inputs,
                &mut cpi_context_account,
                fee_payer,
                remaining_accounts,
            );
            assert!(result.is_ok());
            assert_eq!(
                cpi_context_account.as_ref().unwrap().context.len(),
                i as usize
            );
            assert_eq!(cpi_context_account.as_ref().unwrap().fee_payer, fee_payer);
            clean_input_data(&mut inputs);
            assert_eq!(
                cpi_context_account.as_ref().unwrap().context[(i - 1) as usize],
                inputs
            );
            assert_eq!(result.unwrap(), None);
            assert_eq!(account_info.data, pre_account_info);
        }
        // account info data doesn't change hence we change it manually for the test here
        let mut data = vec![22, 20, 149, 218, 74, 204, 128, 166];
        let mut struct_data = Vec::new();
        cpi_context_account
            .as_ref()
            .unwrap()
            .serialize(&mut struct_data)
            .unwrap();
        data.extend_from_slice(struct_data.as_slice());
        let mut lamports = 0;

        let account_info = AccountInfo {
            key: &Pubkey::new_unique(),
            is_signer: false,
            is_writable: false,
            lamports: RefCell::new(&mut lamports).into(),
            data: RefCell::new(data.as_mut_slice()).into(),
            owner: &crate::ID,
            rent_epoch: 0,
            executable: false,
        };
        let mut cpi_context_account =
            Some(Account::<CpiContextAccount>::try_from(account_info.as_ref()).unwrap());

        let inputs = create_test_instruction_data(false, false, 10);
        let mut input_bytes = Vec::new();
        inputs.serialize(&mut input_bytes).unwrap();
        let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();

        let result = process_cpi_context(
            z_inputs,
            &mut cpi_context_account,
            fee_payer,
            remaining_accounts,
        );
        assert!(result.is_ok());
        let result = result.unwrap().unwrap();

        assert!(result.new_address_params.is_empty());
        for i in 1..10 {
            assert_eq!(
                result.output_compressed_accounts[i]
                    .compressed_account
                    .lamports,
                i as u64
            );
            assert_eq!(
                result.input_compressed_accounts_with_merkle_context[i]
                    .compressed_account
                    .lamports,
                i as u64
            );
        }
        assert_eq!(
            cpi_context_account.as_ref().unwrap().associated_merkle_tree,
            merkle_tree_pubkey
        );
        assert_eq!(
            cpi_context_account.as_ref().unwrap().fee_payer,
            Pubkey::default()
        );
        assert_eq!(cpi_context_account.as_ref().unwrap().context.len(), 0);
    }
}
