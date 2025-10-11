use anchor_lang::prelude::*;
use light_compressed_account::{
    hash_to_bn254_field_size_be,
    instruction_data::{
        compressed_proof::CompressedProof, cpi_context::CompressedCpiContext,
        data::OutputCompressedAccountWithPackedContext, with_readonly::InAccount,
    },
};

use crate::{
    constants::NOT_FROZEN,
    process_transfer::{
        add_data_hash_to_input_compressed_accounts, cpi_execute_compressed_transaction_transfer,
        create_output_compressed_accounts,
        get_input_compressed_accounts_with_merkle_context_and_check_signer,
        InputTokenDataWithContext,
    },
    ErrorCode, GenericInstruction,
};

// TODO: add instruction which accepts an optional proof
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedTokenInstructionDataApprove {
    pub proof: CompressedProof,
    pub mint: Pubkey,
    pub input_token_data_with_context: Vec<InputTokenDataWithContext>,
    pub cpi_context: Option<CompressedCpiContext>,
    pub delegate: Pubkey,
    pub delegated_amount: u64,
    /// Index in remaining accounts.
    pub delegate_merkle_tree_index: u8,
    /// Index in remaining accounts.
    pub change_account_merkle_tree_index: u8,
    pub delegate_lamports: Option<u64>,
}

/// Processes an approve instruction.
/// - creates an output compressed acount which is delegated to the delegate.
/// - creates a change account for the remaining amount (sum inputs - delegated amount).
/// - ignores prior delegations.
/// 1. unpack instruction data and input compressed accounts
/// 2. calculate change amount
/// 3. create output compressed accounts
/// 4. pack token data into input compressed accounts
/// 5. execute compressed transaction
pub fn process_approve<'a, 'b, 'c, 'info: 'b + 'c>(
    ctx: Context<'a, 'b, 'c, 'info, GenericInstruction<'info>>,
    inputs: Vec<u8>,
) -> Result<()> {
    let inputs: CompressedTokenInstructionDataApprove =
        CompressedTokenInstructionDataApprove::deserialize(&mut inputs.as_slice())?;
    // CPI context check not needed: delegation operations don't modify Solana account state
    let (compressed_input_accounts, output_compressed_accounts) =
        create_input_and_output_accounts_approve(
            &inputs,
            &ctx.accounts.authority.key(),
            ctx.remaining_accounts,
        )?;
    let proof = if inputs.proof == CompressedProof::default() {
        None
    } else {
        Some(inputs.proof)
    };
    cpi_execute_compressed_transaction_transfer(
        ctx.accounts,
        compressed_input_accounts,
        output_compressed_accounts,
        false,
        proof,
        inputs.cpi_context,
        ctx.accounts.cpi_authority_pda.to_account_info(),
        ctx.accounts.light_system_program.to_account_info(),
        ctx.accounts.self_program.to_account_info(),
        ctx.remaining_accounts,
    )
}

pub fn create_input_and_output_accounts_approve(
    inputs: &CompressedTokenInstructionDataApprove,
    authority: &Pubkey,
    remaining_accounts: &[AccountInfo<'_>],
) -> Result<(
    Vec<InAccount>,
    Vec<OutputCompressedAccountWithPackedContext>,
)> {
    if inputs.input_token_data_with_context.is_empty() {
        return err!(ErrorCode::NoInputTokenAccountsProvided);
    }
    let (mut compressed_input_accounts, input_token_data, sum_lamports) =
        get_input_compressed_accounts_with_merkle_context_and_check_signer::<NOT_FROZEN>(
            authority,
            &None,
            remaining_accounts,
            &inputs.input_token_data_with_context,
            &inputs.mint,
        )?;
    let sum_inputs = input_token_data.iter().map(|x| x.amount).sum::<u64>();
    let change_amount = match sum_inputs.checked_sub(inputs.delegated_amount) {
        Some(change_amount) => change_amount,
        None => return err!(ErrorCode::ArithmeticUnderflow),
    };

    let delegated_lamports = inputs.delegate_lamports.unwrap_or(0);
    let change_lamports = match sum_lamports.checked_sub(delegated_lamports) {
        Some(change_lamports) => change_lamports,
        None => return err!(ErrorCode::ArithmeticUnderflow),
    };

    let hashed_mint = hash_to_bn254_field_size_be(&inputs.mint.to_bytes());

    let lamports = if sum_lamports != 0 {
        let change_lamports = if change_lamports != 0 {
            Some(change_lamports)
        } else {
            None
        };
        Some(vec![inputs.delegate_lamports, change_lamports])
    } else {
        None
    };
    // Only create outputs if the change amount is not zero.
    let (
        mut output_compressed_accounts,
        pubkeys,
        is_delegate,
        amounts,
        lamports,
        merkle_tree_indices,
    ) = if change_amount > 0 || change_lamports > 0 {
        (
            vec![OutputCompressedAccountWithPackedContext::default(); 2],
            vec![*authority, *authority],
            Some(vec![true, false]),
            vec![inputs.delegated_amount, change_amount],
            lamports,
            vec![
                inputs.delegate_merkle_tree_index,
                inputs.change_account_merkle_tree_index,
            ],
        )
    } else {
        (
            vec![OutputCompressedAccountWithPackedContext::default(); 1],
            vec![*authority],
            Some(vec![true]),
            vec![inputs.delegated_amount],
            lamports,
            vec![inputs.delegate_merkle_tree_index],
        )
    };
    create_output_compressed_accounts(
        &mut output_compressed_accounts,
        inputs.mint,
        pubkeys.as_slice(),
        Some(inputs.delegate),
        is_delegate,
        amounts.as_slice(),
        lamports,
        &hashed_mint,
        &merkle_tree_indices,
        remaining_accounts,
    )?;
    add_data_hash_to_input_compressed_accounts::<NOT_FROZEN>(
        &mut compressed_input_accounts,
        input_token_data.as_slice(),
        &hashed_mint,
        remaining_accounts,
    )?;
    Ok((compressed_input_accounts, output_compressed_accounts))
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedTokenInstructionDataRevoke {
    pub proof: CompressedProof,
    pub mint: Pubkey,
    pub input_token_data_with_context: Vec<InputTokenDataWithContext>,
    pub cpi_context: Option<CompressedCpiContext>,
    pub output_account_merkle_tree_index: u8,
}

pub fn process_revoke<'a, 'b, 'c, 'info: 'b + 'c>(
    ctx: Context<'a, 'b, 'c, 'info, GenericInstruction<'info>>,
    inputs: Vec<u8>,
) -> Result<()> {
    let inputs: CompressedTokenInstructionDataRevoke =
        CompressedTokenInstructionDataRevoke::deserialize(&mut inputs.as_slice())?;
    // CPI context check not needed: delegation operations don't modify Solana account state
    let (compressed_input_accounts, output_compressed_accounts) =
        create_input_and_output_accounts_revoke(
            &inputs,
            &ctx.accounts.authority.key(),
            ctx.remaining_accounts,
        )?;
    let proof = if inputs.proof == CompressedProof::default() {
        None
    } else {
        Some(inputs.proof)
    };
    cpi_execute_compressed_transaction_transfer(
        ctx.accounts,
        compressed_input_accounts,
        output_compressed_accounts,
        false,
        proof,
        inputs.cpi_context,
        ctx.accounts.cpi_authority_pda.to_account_info(),
        ctx.accounts.light_system_program.to_account_info(),
        ctx.accounts.self_program.to_account_info(),
        ctx.remaining_accounts,
    )?;
    Ok(())
}

pub fn create_input_and_output_accounts_revoke(
    inputs: &CompressedTokenInstructionDataRevoke,
    authority: &Pubkey,
    remaining_accounts: &[AccountInfo<'_>],
) -> Result<(
    Vec<InAccount>,
    Vec<OutputCompressedAccountWithPackedContext>,
)> {
    if inputs.input_token_data_with_context.is_empty() {
        return err!(ErrorCode::NoInputTokenAccountsProvided);
    }
    let (mut compressed_input_accounts, input_token_data, sum_lamports) =
        get_input_compressed_accounts_with_merkle_context_and_check_signer::<NOT_FROZEN>(
            authority,
            &None,
            remaining_accounts,
            &inputs.input_token_data_with_context,
            &inputs.mint,
        )?;
    let sum_inputs = input_token_data.iter().map(|x| x.amount).sum::<u64>();
    let lamports = if sum_lamports != 0 {
        Some(vec![Some(sum_lamports)])
    } else {
        None
    };
    let mut output_compressed_accounts =
        vec![OutputCompressedAccountWithPackedContext::default(); 1];
    let hashed_mint = hash_to_bn254_field_size_be(&inputs.mint.to_bytes());

    create_output_compressed_accounts(
        &mut output_compressed_accounts,
        inputs.mint,
        &[*authority; 1],
        None,
        None,
        &[sum_inputs],
        lamports,
        &hashed_mint,
        &[inputs.output_account_merkle_tree_index],
        remaining_accounts,
    )?;
    add_data_hash_to_input_compressed_accounts::<NOT_FROZEN>(
        &mut compressed_input_accounts,
        input_token_data.as_slice(),
        &hashed_mint,
        remaining_accounts,
    )?;
    Ok((compressed_input_accounts, output_compressed_accounts))
}

#[cfg(not(target_os = "solana"))]
pub mod sdk {

    use std::result::Result;

    use anchor_lang::{AnchorSerialize, InstructionData, ToAccountMetas};
    use light_compressed_account::compressed_account::{CompressedAccount, MerkleContext};
    use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

    use super::*;
    use crate::{
        process_transfer::{
            get_cpi_authority_pda,
            transfer_sdk::{
                create_input_output_and_remaining_accounts, to_account_metas, TransferSdkError,
            },
        },
        TokenData,
    };

    pub struct CreateApproveInstructionInputs {
        pub fee_payer: Pubkey,
        pub authority: Pubkey,
        pub root_indices: Vec<Option<u16>>,
        pub proof: CompressedProof,
        pub input_token_data: Vec<TokenData>,
        pub input_compressed_accounts: Vec<CompressedAccount>,
        pub input_merkle_contexts: Vec<MerkleContext>,
        pub mint: Pubkey,
        pub delegated_amount: u64,
        pub delegate_lamports: Option<u64>,
        pub delegated_compressed_account_merkle_tree: Pubkey,
        pub change_compressed_account_merkle_tree: Pubkey,
        pub delegate: Pubkey,
    }

    pub fn create_approve_instruction(
        inputs: CreateApproveInstructionInputs,
    ) -> Result<Instruction, TransferSdkError> {
        let (remaining_accounts, input_token_data_with_context, _) =
            create_input_output_and_remaining_accounts(
                &[
                    inputs.delegated_compressed_account_merkle_tree,
                    inputs.change_compressed_account_merkle_tree,
                ],
                &inputs.input_token_data,
                &inputs.input_compressed_accounts,
                &inputs.input_merkle_contexts,
                &inputs.root_indices,
                &Vec::new(),
            );
        let delegated_merkle_tree_index =
            match remaining_accounts.get(&inputs.delegated_compressed_account_merkle_tree) {
                Some(delegated_merkle_tree_index) => delegated_merkle_tree_index,
                None => return Err(TransferSdkError::AccountNotFound),
            };
        let change_account_merkle_tree_index =
            match remaining_accounts.get(&inputs.change_compressed_account_merkle_tree) {
                Some(change_account_merkle_tree_index) => change_account_merkle_tree_index,
                None => return Err(TransferSdkError::AccountNotFound),
            };
        let inputs_struct = CompressedTokenInstructionDataApprove {
            proof: inputs.proof,
            mint: inputs.mint,
            input_token_data_with_context,
            cpi_context: None,
            delegate: inputs.delegate,
            delegated_amount: inputs.delegated_amount,
            delegate_merkle_tree_index: *delegated_merkle_tree_index as u8,
            change_account_merkle_tree_index: *change_account_merkle_tree_index as u8,
            delegate_lamports: inputs.delegate_lamports,
        };
        let remaining_accounts = to_account_metas(remaining_accounts);
        let mut serialized_ix_data = Vec::new();
        CompressedTokenInstructionDataApprove::serialize(&inputs_struct, &mut serialized_ix_data)
            .map_err(|_| TransferSdkError::SerializationError)?;

        let (cpi_authority_pda, _) = get_cpi_authority_pda();
        let instruction_data = crate::instruction::Approve {
            inputs: serialized_ix_data,
        };

        let accounts = crate::accounts::GenericInstruction {
            fee_payer: inputs.fee_payer,
            authority: inputs.authority,
            cpi_authority_pda,
            light_system_program: light_system_program::ID,
            registered_program_pda: light_system_program::utils::get_registered_program_pda(
                &light_system_program::ID,
            ),
            noop_program: Pubkey::new_from_array(
                account_compression::utils::constants::NOOP_PUBKEY,
            ),
            account_compression_authority: light_system_program::utils::get_cpi_authority_pda(
                &light_system_program::ID,
            ),
            account_compression_program: account_compression::ID,
            self_program: crate::ID,
            system_program: solana_sdk::system_program::ID,
        };

        Ok(Instruction {
            program_id: crate::ID,
            accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),

            data: instruction_data.data(),
        })
    }

    pub struct CreateRevokeInstructionInputs {
        pub fee_payer: Pubkey,
        pub authority: Pubkey,
        pub root_indices: Vec<Option<u16>>,
        pub proof: CompressedProof,
        pub input_token_data: Vec<TokenData>,
        pub input_compressed_accounts: Vec<CompressedAccount>,
        pub input_merkle_contexts: Vec<MerkleContext>,
        pub mint: Pubkey,
        pub output_account_merkle_tree: Pubkey,
    }

    pub fn create_revoke_instruction(
        inputs: CreateRevokeInstructionInputs,
    ) -> Result<Instruction, TransferSdkError> {
        let (remaining_accounts, input_token_data_with_context, _) =
            create_input_output_and_remaining_accounts(
                &[inputs.output_account_merkle_tree],
                &inputs.input_token_data,
                &inputs.input_compressed_accounts,
                &inputs.input_merkle_contexts,
                &inputs.root_indices,
                &Vec::new(),
            );
        let output_account_merkle_tree_index =
            match remaining_accounts.get(&inputs.output_account_merkle_tree) {
                Some(output_account_merkle_tree_index) => output_account_merkle_tree_index,
                None => return Err(TransferSdkError::AccountNotFound),
            };

        let inputs_struct = CompressedTokenInstructionDataRevoke {
            proof: inputs.proof,
            mint: inputs.mint,
            input_token_data_with_context,
            cpi_context: None,
            output_account_merkle_tree_index: *output_account_merkle_tree_index as u8,
        };
        let remaining_accounts = to_account_metas(remaining_accounts);
        let mut serialized_ix_data = Vec::new();
        CompressedTokenInstructionDataRevoke::serialize(&inputs_struct, &mut serialized_ix_data)
            .map_err(|_| TransferSdkError::SerializationError)?;

        let (cpi_authority_pda, _) = get_cpi_authority_pda();
        let instruction_data = crate::instruction::Revoke {
            inputs: serialized_ix_data,
        };

        let accounts = crate::accounts::GenericInstruction {
            fee_payer: inputs.fee_payer,
            authority: inputs.authority,
            cpi_authority_pda,
            light_system_program: light_system_program::ID,
            registered_program_pda: light_system_program::utils::get_registered_program_pda(
                &light_system_program::ID,
            ),
            noop_program: Pubkey::new_from_array(
                account_compression::utils::constants::NOOP_PUBKEY,
            ),
            account_compression_authority: light_system_program::utils::get_cpi_authority_pda(
                &light_system_program::ID,
            ),
            account_compression_program: account_compression::ID,
            self_program: crate::ID,
            system_program: solana_sdk::system_program::ID,
        };

        Ok(Instruction {
            program_id: crate::ID,
            accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),

            data: instruction_data.data(),
        })
    }
}

#[cfg(test)]
mod test {
    use account_compression::StateMerkleTreeAccount;
    use anchor_lang::{solana_program::account_info::AccountInfo, Discriminator};
    use light_compressed_account::compressed_account::PackedMerkleContext;
    use light_ctoken_types::state::CompressedTokenAccountState;

    use super::*;
    use crate::{freeze::test_freeze::create_expected_token_output_accounts, TokenData};

    // TODO: add randomized and edge case tests
    #[test]
    fn test_approve() {
        let merkle_tree_pubkey = Pubkey::new_unique();
        let mut merkle_tree_account_lamports = 0;
        let mut merkle_tree_account_data = StateMerkleTreeAccount::DISCRIMINATOR.to_vec();
        let merkle_tree_pubkey_1 = Pubkey::new_unique();
        let mut merkle_tree_account_lamports_1 = 0;
        let mut merkle_tree_account_data_1 = StateMerkleTreeAccount::DISCRIMINATOR.to_vec();
        let nullifier_queue_pubkey = Pubkey::new_unique();
        let mut nullifier_queue_account_lamports = 0;
        let mut nullifier_queue_account_data = Vec::new();
        let remaining_accounts = vec![
            AccountInfo::new(
                &merkle_tree_pubkey,
                false,
                false,
                &mut merkle_tree_account_lamports,
                &mut merkle_tree_account_data,
                &account_compression::ID,
                false,
                0,
            ),
            AccountInfo::new(
                &nullifier_queue_pubkey,
                false,
                false,
                &mut nullifier_queue_account_lamports,
                &mut nullifier_queue_account_data,
                &account_compression::ID,
                false,
                0,
            ),
            AccountInfo::new(
                &merkle_tree_pubkey_1,
                false,
                false,
                &mut merkle_tree_account_lamports_1,
                &mut merkle_tree_account_data_1,
                &account_compression::ID,
                false,
                0,
            ),
        ];
        let authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();
        let input_token_data_with_context = vec![
            InputTokenDataWithContext {
                amount: 100,
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    queue_pubkey_index: 1,
                    leaf_index: 1,
                    prove_by_index: false,
                },
                root_index: 0,
                delegate_index: Some(1),
                lamports: None,
                tlv: None,
            },
            InputTokenDataWithContext {
                amount: 101,
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    queue_pubkey_index: 1,
                    leaf_index: 2,
                    prove_by_index: false,
                },
                root_index: 0,
                delegate_index: None,
                lamports: None,
                tlv: None,
            },
        ];
        let inputs = CompressedTokenInstructionDataApprove {
            proof: CompressedProof::default(),
            mint,
            input_token_data_with_context,
            cpi_context: None,
            delegate,
            delegated_amount: 50,
            delegate_merkle_tree_index: 0,
            change_account_merkle_tree_index: 2,
            delegate_lamports: None,
        };
        let (compressed_input_accounts, output_compressed_accounts) =
            create_input_and_output_accounts_approve(&inputs, &authority, &remaining_accounts)
                .unwrap();
        assert_eq!(compressed_input_accounts.len(), 2);
        assert_eq!(output_compressed_accounts.len(), 2);
        let expected_change_token_data = TokenData {
            mint: mint.into(),
            owner: authority.into(),
            amount: 151,
            delegate: None,
            state: CompressedTokenAccountState::Initialized as u8,
            tlv: None,
        };
        let expected_delegated_token_data = TokenData {
            mint: mint.into(),
            owner: authority.into(),
            amount: 50,
            delegate: Some(delegate.into()),
            state: CompressedTokenAccountState::Initialized as u8,
            tlv: None,
        };
        let expected_compressed_output_accounts = create_expected_token_output_accounts(
            vec![expected_delegated_token_data, expected_change_token_data],
            vec![0, 2],
        );

        assert_eq!(
            output_compressed_accounts,
            expected_compressed_output_accounts
        );
    }

    #[test]
    fn test_revoke() {
        let merkle_tree_pubkey = Pubkey::new_unique();
        let mut merkle_tree_account_lamports = 0;
        let mut merkle_tree_account_data = StateMerkleTreeAccount::DISCRIMINATOR.to_vec();
        let nullifier_queue_pubkey = Pubkey::new_unique();
        let mut nullifier_queue_account_lamports = 0;
        let mut nullifier_queue_account_data = Vec::new();
        let merkle_tree_pubkey_1 = Pubkey::new_unique();
        let mut merkle_tree_account_lamports_1 = 0;
        let mut merkle_tree_account_data_1 = StateMerkleTreeAccount::DISCRIMINATOR.to_vec();
        let remaining_accounts = vec![
            AccountInfo::new(
                &merkle_tree_pubkey,
                false,
                false,
                &mut merkle_tree_account_lamports,
                &mut merkle_tree_account_data,
                &account_compression::ID,
                false,
                0,
            ),
            AccountInfo::new(
                &nullifier_queue_pubkey,
                false,
                false,
                &mut nullifier_queue_account_lamports,
                &mut nullifier_queue_account_data,
                &account_compression::ID,
                false,
                0,
            ),
            AccountInfo::new(
                &merkle_tree_pubkey_1,
                false,
                false,
                &mut merkle_tree_account_lamports_1,
                &mut merkle_tree_account_data_1,
                &account_compression::ID,
                false,
                0,
            ),
        ];
        let authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let input_token_data_with_context = vec![
            InputTokenDataWithContext {
                amount: 100,

                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    queue_pubkey_index: 1,
                    leaf_index: 1,
                    prove_by_index: false,
                },
                root_index: 0,
                delegate_index: Some(1), // Doesn't matter it is not checked if the proof is not verified
                lamports: None,
                tlv: None,
            },
            InputTokenDataWithContext {
                amount: 101,

                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    queue_pubkey_index: 1,
                    leaf_index: 2,
                    prove_by_index: false,
                },
                root_index: 0,
                delegate_index: Some(1), // Doesn't matter it is not checked if the proof is not verified
                lamports: None,
                tlv: None,
            },
        ];
        let inputs = CompressedTokenInstructionDataRevoke {
            proof: CompressedProof::default(),
            mint,
            input_token_data_with_context,
            cpi_context: None,
            output_account_merkle_tree_index: 2,
        };
        let (compressed_input_accounts, output_compressed_accounts) =
            create_input_and_output_accounts_revoke(&inputs, &authority, &remaining_accounts)
                .unwrap();
        assert_eq!(compressed_input_accounts.len(), 2);
        assert_eq!(output_compressed_accounts.len(), 1);
        let expected_change_token_data = TokenData {
            mint: mint.into(),
            owner: authority.into(),
            amount: 201,
            delegate: None,
            state: CompressedTokenAccountState::Initialized as u8,
            tlv: None,
        };
        let expected_compressed_output_accounts =
            create_expected_token_output_accounts(vec![expected_change_token_data], vec![2]);
        assert_eq!(
            output_compressed_accounts,
            expected_compressed_output_accounts
        );

        let lamports_amount = 2;

        let input_token_data_with_context = vec![
            InputTokenDataWithContext {
                amount: 100,

                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    queue_pubkey_index: 1,
                    leaf_index: 1,
                    prove_by_index: false,
                },
                root_index: 0,
                delegate_index: Some(1), // Doesn't matter it is not checked if the proof is not verified
                lamports: Some(lamports_amount / 2),
                tlv: None,
            },
            InputTokenDataWithContext {
                amount: 101,

                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    queue_pubkey_index: 1,
                    leaf_index: 2,
                    prove_by_index: false,
                },
                root_index: 0,
                delegate_index: Some(1), // Doesn't matter it is not checked if the proof is not verified
                lamports: Some(lamports_amount / 2),
                tlv: None,
            },
        ];
        let inputs = CompressedTokenInstructionDataRevoke {
            proof: CompressedProof::default(),
            mint,
            input_token_data_with_context,
            cpi_context: None,
            output_account_merkle_tree_index: 2,
        };
        let (compressed_input_accounts, output_compressed_accounts) =
            create_input_and_output_accounts_revoke(&inputs, &authority, &remaining_accounts)
                .unwrap();
        assert_eq!(compressed_input_accounts.len(), 2);
        assert_eq!(output_compressed_accounts.len(), 1);
        let expected_change_token_data = TokenData {
            mint: mint.into(),
            owner: authority.into(),
            amount: 201,
            delegate: None,
            state: CompressedTokenAccountState::Initialized as u8,
            tlv: None,
        };
        let mut expected_compressed_output_accounts =
            create_expected_token_output_accounts(vec![expected_change_token_data], vec![2]);
        expected_compressed_output_accounts[0]
            .compressed_account
            .lamports = lamports_amount;
        assert_eq!(
            output_compressed_accounts,
            expected_compressed_output_accounts
        );
    }
}
