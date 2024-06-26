use anchor_lang::prelude::*;
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::{compressed_account::PackedCompressedAccountWithMerkleContext, CompressedCpiContext},
    OutputCompressedAccountWithPackedContext,
};
use light_utils::hash_to_bn254_field_size_be;

use crate::{
    process_transfer::{
        add_token_data_to_input_compressed_accounts, cpi_execute_compressed_transaction_transfer,
        create_output_compressed_accounts,
        get_input_compressed_accounts_with_merkle_context_and_check_signer,
        InputTokenDataWithContext,
    },
    ErrorCode, GenericInstruction,
};

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedTokenInstructionDataApprove {
    pub proof: CompressedProof,
    pub mint: Pubkey,
    pub input_token_data_with_context: Vec<InputTokenDataWithContext>,
    pub cpi_context: Option<CompressedCpiContext>,
    pub delegate: Pubkey,
    pub delegated_amount: u64,
    pub delegate_merkle_tree_index: u8,
    pub change_account_merkle_tree_index: u8,
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
    let (compressed_input_accounts, output_compressed_accounts) =
        create_input_and_output_accounts_approve(
            &inputs,
            &ctx.accounts.authority.key(),
            ctx.remaining_accounts,
        )?;
    cpi_execute_compressed_transaction_transfer(
        ctx.accounts,
        compressed_input_accounts,
        &output_compressed_accounts,
        Some(inputs.proof),
        inputs.cpi_context,
        ctx.accounts.cpi_authority_pda.to_account_info(),
        ctx.accounts.light_system_program.to_account_info(),
        ctx.accounts.self_program.to_account_info(),
        ctx.remaining_accounts,
    )?;
    Ok(())
}

pub fn create_input_and_output_accounts_approve(
    inputs: &CompressedTokenInstructionDataApprove,
    authority: &Pubkey,
    remaining_accounts: &[AccountInfo<'_>],
) -> Result<(
    Vec<PackedCompressedAccountWithMerkleContext>,
    Vec<OutputCompressedAccountWithPackedContext>,
)> {
    let (mut compressed_input_accounts, input_token_data) =
        get_input_compressed_accounts_with_merkle_context_and_check_signer::<false>(
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
    let mut output_compressed_accounts =
        vec![OutputCompressedAccountWithPackedContext::default(); 2];
    let hashed_mint = match hash_to_bn254_field_size_be(&inputs.mint.to_bytes()) {
        Some(hashed_mint) => hashed_mint.0,
        None => return err!(ErrorCode::HashToFieldError),
    };

    create_output_compressed_accounts(
        &mut output_compressed_accounts,
        inputs.mint,
        &[*authority; 2],
        Some(inputs.delegate),
        Some(vec![true, false]),
        &[inputs.delegated_amount, change_amount],
        None,
        &hashed_mint,
        &[
            inputs.delegate_merkle_tree_index,
            inputs.change_account_merkle_tree_index,
        ],
    )?;
    add_token_data_to_input_compressed_accounts::<false>(
        &mut compressed_input_accounts,
        input_token_data.as_slice(),
        &hashed_mint,
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
    let (compressed_input_accounts, output_compressed_accounts) =
        create_input_and_output_accounts_revoke(
            &inputs,
            &ctx.accounts.authority.key(),
            ctx.remaining_accounts,
        )?;
    cpi_execute_compressed_transaction_transfer(
        ctx.accounts,
        compressed_input_accounts,
        &output_compressed_accounts,
        Some(inputs.proof),
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
    Vec<PackedCompressedAccountWithMerkleContext>,
    Vec<OutputCompressedAccountWithPackedContext>,
)> {
    let (mut compressed_input_accounts, input_token_data) =
        get_input_compressed_accounts_with_merkle_context_and_check_signer::<false>(
            authority,
            &None,
            remaining_accounts,
            &inputs.input_token_data_with_context,
            &inputs.mint,
        )?;
    let sum_inputs = input_token_data.iter().map(|x| x.amount).sum::<u64>();
    let mut output_compressed_accounts =
        vec![OutputCompressedAccountWithPackedContext::default(); 1];
    let hashed_mint = match hash_to_bn254_field_size_be(&inputs.mint.to_bytes()) {
        Some(hashed_mint) => hashed_mint.0,
        None => return err!(ErrorCode::HashToFieldError),
    };

    create_output_compressed_accounts(
        &mut output_compressed_accounts,
        inputs.mint,
        &[*authority; 1],
        None,
        None,
        &[sum_inputs],
        None, // TODO: add wrapped sol support
        &hashed_mint,
        &[inputs.output_account_merkle_tree_index],
    )?;
    add_token_data_to_input_compressed_accounts::<false>(
        &mut compressed_input_accounts,
        input_token_data.as_slice(),
        &hashed_mint,
    )?;
    Ok((compressed_input_accounts, output_compressed_accounts))
}

#[cfg(not(target_os = "solana"))]
pub mod sdk {

    use anchor_lang::{AnchorSerialize, InstructionData, ToAccountMetas};
    use light_system_program::{
        invoke::processor::CompressedProof, sdk::compressed_account::MerkleContext,
    };
    use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

    use crate::{
        process_transfer::{
            get_cpi_authority_pda,
            transfer_sdk::{
                create_input_output_and_remaining_accounts, to_account_metas, TransferSdkError,
            },
        },
        token_data::TokenData,
    };

    use super::{CompressedTokenInstructionDataApprove, CompressedTokenInstructionDataRevoke};

    pub struct CreateApproveInstructionInputs {
        pub fee_payer: Pubkey,
        pub authority: Pubkey,
        pub root_indices: Vec<u16>,
        pub proof: CompressedProof,
        pub input_token_data: Vec<TokenData>,
        pub input_merkle_contexts: Vec<MerkleContext>,
        pub mint: Pubkey,
        pub delegated_amount: u64,
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
        pub root_indices: Vec<u16>,
        pub proof: CompressedProof,
        pub input_token_data: Vec<TokenData>,
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
    use super::*;
    use crate::{
        freeze::test_freeze::create_expected_token_output_accounts, token_data::AccountState,
        TokenData,
    };
    use anchor_lang::solana_program::account_info::AccountInfo;
    use light_system_program::sdk::compressed_account::PackedMerkleContext;

    // TODO: add randomized and edge case tests
    #[test]
    fn test_approve() {
        let merkle_tree_pubkey = Pubkey::new_unique();
        let mut merkle_tree_account_lamports = 0;
        let mut merkle_tree_account_data = Vec::new();
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
        ];
        let authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();
        let input_token_data_with_context = vec![
            InputTokenDataWithContext {
                amount: 100,

                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    nullifier_queue_pubkey_index: 1,
                    leaf_index: 1,
                },
                root_index: 0,
                delegate_index: Some(1),
                lamports: None,
            },
            InputTokenDataWithContext {
                amount: 101,

                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    nullifier_queue_pubkey_index: 1,
                    leaf_index: 2,
                },
                root_index: 0,
                delegate_index: None,
                lamports: None,
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
            change_account_merkle_tree_index: 1,
        };
        let (compressed_input_accounts, output_compressed_accounts) =
            create_input_and_output_accounts_approve(&inputs, &authority, &remaining_accounts)
                .unwrap();
        assert_eq!(compressed_input_accounts.len(), 2);
        assert_eq!(output_compressed_accounts.len(), 2);
        let expected_change_token_data = TokenData {
            mint,
            owner: authority,
            amount: 151,
            delegate: None,
            state: AccountState::Initialized,
        };
        let expected_delegated_token_data = TokenData {
            mint,
            owner: authority,
            amount: 50,
            delegate: Some(delegate),
            state: AccountState::Initialized,
        };
        let expected_compressed_output_accounts = create_expected_token_output_accounts(
            vec![expected_delegated_token_data, expected_change_token_data],
            vec![0, 1],
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
        let mut merkle_tree_account_data = Vec::new();
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
        ];
        let authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let input_token_data_with_context = vec![
            InputTokenDataWithContext {
                amount: 100,

                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    nullifier_queue_pubkey_index: 1,
                    leaf_index: 1,
                },
                root_index: 0,
                delegate_index: Some(1), // Doesn't matter it is not checked if the proof is not verified
                lamports: None,
            },
            InputTokenDataWithContext {
                amount: 101,

                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    nullifier_queue_pubkey_index: 1,
                    leaf_index: 2,
                },
                root_index: 0,
                delegate_index: Some(1), // Doesn't matter it is not checked if the proof is not verified
                lamports: None,
            },
        ];
        let inputs = CompressedTokenInstructionDataRevoke {
            proof: CompressedProof::default(),
            mint,
            input_token_data_with_context,
            cpi_context: None,
            output_account_merkle_tree_index: 1,
        };
        let (compressed_input_accounts, output_compressed_accounts) =
            create_input_and_output_accounts_revoke(&inputs, &authority, &remaining_accounts)
                .unwrap();
        assert_eq!(compressed_input_accounts.len(), 2);
        assert_eq!(output_compressed_accounts.len(), 1);
        let expected_change_token_data = TokenData {
            mint,
            owner: authority,
            amount: 201,
            delegate: None,
            state: AccountState::Initialized,
        };
        let expected_compressed_output_accounts =
            create_expected_token_output_accounts(vec![expected_change_token_data], vec![1]);
        assert_eq!(
            output_compressed_accounts,
            expected_compressed_output_accounts
        );
    }
}
