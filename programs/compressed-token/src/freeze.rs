use anchor_lang::prelude::*;
use light_hasher::DataHasher;
use light_hasher::Poseidon;
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::{
        compressed_account::{
            CompressedAccount, CompressedAccountData, PackedCompressedAccountWithMerkleContext,
        },
        CompressedCpiContext,
    },
    OutputCompressedAccountWithPackedContext,
};
use light_utils::hash_to_bn254_field_size_be;

use crate::{
    constants::TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
    process_transfer::{
        add_token_data_to_input_compressed_accounts, cpi_execute_compressed_transaction_transfer,
        get_input_compressed_accounts_with_merkle_context_and_check_signer,
        InputTokenDataWithContext,
    },
    token_data::{AccountState, TokenData},
    FreezeInstruction,
};

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedTokenInstructionDataFreeze {
    pub proof: CompressedProof,
    pub owner: Pubkey,
    pub input_token_data_with_context: Vec<InputTokenDataWithContext>,
    pub cpi_context: Option<CompressedCpiContext>,
    pub outputs_merkle_tree_index: u8,
}

pub fn process_freeze_or_thaw<
    'a,
    'b,
    'c,
    'info: 'b + 'c,
    const FROZEN_INPUTS: bool,
    const FROZEN_OUTPUTS: bool,
>(
    ctx: Context<'a, 'b, 'c, 'info, FreezeInstruction<'info>>,
    inputs: Vec<u8>,
) -> Result<()> {
    let inputs: CompressedTokenInstructionDataFreeze =
        CompressedTokenInstructionDataFreeze::deserialize(&mut inputs.as_slice())?;
    let (compressed_input_accounts, output_compressed_accounts) =
        create_input_and_output_accounts_freeze_or_thaw::<FROZEN_INPUTS, FROZEN_OUTPUTS>(
            &inputs,
            &ctx.accounts.mint.key(),
            ctx.remaining_accounts,
        )?;
    // TODO: implement trait for TransferInstruction and FreezeInstruction
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

pub fn create_input_and_output_accounts_freeze_or_thaw<
    const FROZEN_INPUTS: bool,
    const FROZEN_OUTPUTS: bool,
>(
    inputs: &CompressedTokenInstructionDataFreeze,
    mint: &Pubkey,
    remaining_accounts: &[AccountInfo<'_>],
) -> Result<(
    Vec<PackedCompressedAccountWithMerkleContext>,
    Vec<OutputCompressedAccountWithPackedContext>,
)> {
    let (mut compressed_input_accounts, input_token_data) =
        get_input_compressed_accounts_with_merkle_context_and_check_signer::<FROZEN_INPUTS>(
            &inputs.owner,
            &None,
            remaining_accounts,
            &inputs.input_token_data_with_context,
            mint,
        )?;
    let output_len = compressed_input_accounts.len();
    let mut output_compressed_accounts =
        vec![OutputCompressedAccountWithPackedContext::default(); output_len];
    let hashed_mint = hash_to_bn254_field_size_be(mint.to_bytes().as_slice())
        .unwrap()
        .0;
    create_token_output_accounts::<FROZEN_OUTPUTS>(
        inputs.input_token_data_with_context.as_slice(),
        remaining_accounts,
        mint,
        &inputs.owner,
        &inputs.outputs_merkle_tree_index,
        &mut output_compressed_accounts,
    )?;

    add_token_data_to_input_compressed_accounts::<FROZEN_INPUTS>(
        &mut compressed_input_accounts,
        input_token_data.as_slice(),
        &hashed_mint,
    )?;
    Ok((compressed_input_accounts, output_compressed_accounts))
}

fn create_token_output_accounts<const IS_FROZEN: bool>(
    input_token_data_with_context: &[InputTokenDataWithContext],
    remaining_accounts: &[AccountInfo],
    mint: &Pubkey,
    owner: &Pubkey,
    outputs_merkle_tree_index: &u8,
    output_compressed_accounts: &mut [OutputCompressedAccountWithPackedContext],
) -> Result<()> {
    for (i, token_data) in input_token_data_with_context.iter().enumerate() {
        // 83 =
        //      32  mint
        // +    32  owner
        // +    8   amount
        // +    1   delegate
        // +    1   state
        // +    8   delegated_amount
        let mut token_data_bytes = Vec::with_capacity(83);
        let delegate = token_data
            .delegate_index
            .map(|index| remaining_accounts[index as usize].key());
        let state = if IS_FROZEN {
            AccountState::Frozen
        } else {
            AccountState::Initialized
        };
        // 1,000 CU token data and serialize
        let token_data = TokenData {
            mint: *mint,
            owner: *owner,
            amount: token_data.amount,
            delegate,
            state,
            is_native: None,
        };
        token_data.serialize(&mut token_data_bytes).unwrap();

        let data_hash = token_data.hash::<Poseidon>().map_err(ProgramError::from)?;
        let data: CompressedAccountData = CompressedAccountData {
            discriminator: TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
            data: token_data_bytes,
            data_hash,
        };
        // TODO: support wrapped sol
        // let lamports = lamports.and_then(|lamports| lamports[i]).unwrap_or(0);

        output_compressed_accounts[i] = OutputCompressedAccountWithPackedContext {
            compressed_account: CompressedAccount {
                owner: crate::ID,
                lamports: 0,
                data: Some(data),
                address: None,
            },
            merkle_tree_index: *outputs_merkle_tree_index,
        };
    }
    Ok(())
}
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedTokenInstructionDataThaw {
    pub proof: CompressedProof,
    pub owner: Pubkey,
    pub input_token_data_with_context: Vec<InputTokenDataWithContext>,
    pub cpi_context: Option<CompressedCpiContext>,
    pub outputs_merkle_tree_index: u8,
}

#[cfg(not(target_os = "solana"))]
pub mod sdk {

    use anchor_lang::{AnchorSerialize, InstructionData, ToAccountMetas};
    use light_system_program::{
        invoke::processor::CompressedProof, sdk::compressed_account::MerkleContext,
    };
    use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

    use crate::{
        process_transfer::transfer_sdk::{
            create_input_output_and_remaining_accounts, to_account_metas, TransferSdkError,
        },
        token_data::TokenData,
    };

    use super::CompressedTokenInstructionDataFreeze;

    pub struct CreateInstructionInputs {
        pub fee_payer: Pubkey,
        pub authority: Pubkey,
        pub root_indices: Vec<u16>,
        pub proof: CompressedProof,
        pub input_token_data: Vec<TokenData>,
        pub input_merkle_contexts: Vec<MerkleContext>,
        pub outputs_merkle_tree: Pubkey,
    }

    pub fn create_instruction<const FREEZE: bool>(
        inputs: CreateInstructionInputs,
    ) -> Result<Instruction, TransferSdkError> {
        let (remaining_accounts, input_token_data_with_context, _) =
            create_input_output_and_remaining_accounts(
                &[inputs.outputs_merkle_tree],
                &inputs.input_token_data,
                &inputs.input_merkle_contexts,
                &inputs.root_indices,
                &Vec::new(),
            );
        let outputs_merkle_tree_index =
            remaining_accounts.get(&inputs.outputs_merkle_tree).unwrap();

        let inputs_struct = CompressedTokenInstructionDataFreeze {
            proof: inputs.proof,
            input_token_data_with_context,
            cpi_context: None,
            outputs_merkle_tree_index: *outputs_merkle_tree_index as u8,
            owner: inputs.input_token_data[0].owner,
        };
        let remaining_accounts = to_account_metas(remaining_accounts);
        let mut serialized_ix_data = Vec::new();
        CompressedTokenInstructionDataFreeze::serialize(&inputs_struct, &mut serialized_ix_data)
            .unwrap();

        let (cpi_authority_pda, _) = crate::process_transfer::get_cpi_authority_pda();
        let data = if FREEZE {
            crate::instruction::Freeze {
                inputs: serialized_ix_data,
            }
            .data()
        } else {
            crate::instruction::Thaw {
                inputs: serialized_ix_data,
            }
            .data()
        };

        let accounts = crate::accounts::FreezeInstruction {
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
            mint: inputs.input_token_data[0].mint,
        };

        Ok(Instruction {
            program_id: crate::ID,
            accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),

            data,
        })
    }

    pub fn create_freeze_instruction(
        inputs: CreateInstructionInputs,
    ) -> Result<Instruction, TransferSdkError> {
        create_instruction::<true>(inputs)
    }

    pub fn create_thaw_instruction(
        inputs: CreateInstructionInputs,
    ) -> Result<Instruction, TransferSdkError> {
        create_instruction::<false>(inputs)
    }
}

#[cfg(test)]
pub mod test_freeze {
    use crate::{
        constants::TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR, token_data::AccountState, TokenData,
    };

    use super::*;
    use anchor_lang::solana_program::account_info::AccountInfo;
    use light_hasher::{DataHasher, Poseidon};
    use light_system_program::sdk::compressed_account::{
        CompressedAccount, CompressedAccountData, PackedMerkleContext,
    };

    // TODO: add randomized and edge case tests
    #[test]
    fn test_freeze() {
        let merkle_tree_pubkey = Pubkey::new_unique();
        let mut merkle_tree_account_lamports = 0;
        let mut merkle_tree_account_data = Vec::new();
        let nullifier_queue_pubkey = Pubkey::new_unique();
        let mut nullifier_queue_account_lamports = 0;
        let mut nullifier_queue_account_data = Vec::new();
        let delegate = Pubkey::new_unique();
        let mut delegate_account_lamports = 0;
        let mut delegate_account_data = Vec::new();
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
                &delegate,
                false,
                false,
                &mut delegate_account_lamports,
                &mut delegate_account_data,
                &account_compression::ID,
                false,
                0,
            ),
        ];
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        let input_token_data_with_context = vec![
            InputTokenDataWithContext {
                amount: 100,
                is_native: None,
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    nullifier_queue_pubkey_index: 1,
                    leaf_index: 1,
                },
                root_index: 0,
                delegate_index: None,
            },
            InputTokenDataWithContext {
                amount: 101,
                is_native: None,
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    nullifier_queue_pubkey_index: 1,
                    leaf_index: 2,
                },
                root_index: 0,
                delegate_index: Some(2),
            },
        ];
        // Freeze
        {
            let inputs = CompressedTokenInstructionDataFreeze {
                proof: CompressedProof::default(),
                owner,
                input_token_data_with_context: input_token_data_with_context.clone(),
                cpi_context: None,
                outputs_merkle_tree_index: 1,
            };
            let (compressed_input_accounts, output_compressed_accounts) =
                create_input_and_output_accounts_freeze_or_thaw::<false, true>(
                    &inputs,
                    &mint,
                    &remaining_accounts,
                )
                .unwrap();
            assert_eq!(compressed_input_accounts.len(), 2);
            assert_eq!(output_compressed_accounts.len(), 2);
            let expected_change_token_data = TokenData {
                mint,
                owner,
                amount: 100,
                delegate: None,
                state: AccountState::Frozen,
                is_native: None,
            };
            let expected_delegated_token_data = TokenData {
                mint,
                owner,
                amount: 101,
                delegate: Some(delegate),
                state: AccountState::Frozen,
                is_native: None,
            };

            let expected_compressed_output_accounts = create_expected_token_output_accounts(
                vec![expected_change_token_data, expected_delegated_token_data],
                vec![1u8; 2],
            );
            assert_eq!(
                output_compressed_accounts,
                expected_compressed_output_accounts
            );
        }
        // Thaw
        {
            let inputs = CompressedTokenInstructionDataFreeze {
                proof: CompressedProof::default(),
                owner,
                input_token_data_with_context,
                cpi_context: None,
                outputs_merkle_tree_index: 1,
            };
            let (compressed_input_accounts, output_compressed_accounts) =
                create_input_and_output_accounts_freeze_or_thaw::<true, false>(
                    &inputs,
                    &mint,
                    &remaining_accounts,
                )
                .unwrap();
            assert_eq!(compressed_input_accounts.len(), 2);
            assert_eq!(output_compressed_accounts.len(), 2);
            let expected_change_token_data = TokenData {
                mint,
                owner,
                amount: 100,
                delegate: None,
                state: AccountState::Initialized,
                is_native: None,
            };
            let expected_delegated_token_data = TokenData {
                mint,
                owner,
                amount: 101,
                delegate: Some(delegate),
                state: AccountState::Initialized,
                is_native: None,
            };

            let expected_compressed_output_accounts = create_expected_token_output_accounts(
                vec![expected_change_token_data, expected_delegated_token_data],
                vec![1u8; 2],
            );
            assert_eq!(
                output_compressed_accounts,
                expected_compressed_output_accounts
            );
            for account in compressed_input_accounts {
                let account_data = account.compressed_account.data.unwrap();
                let token_data = TokenData::try_from_slice(&account_data.data).unwrap();
                assert_eq!(token_data.state, AccountState::Frozen);
            }
        }
    }

    pub fn create_expected_token_output_accounts(
        expected_token_data: Vec<TokenData>,
        merkle_tree_indices: Vec<u8>,
    ) -> Vec<OutputCompressedAccountWithPackedContext> {
        let mut expected_compressed_output_accounts = Vec::new();
        for (token_data, merkle_tree_index) in
            expected_token_data.iter().zip(merkle_tree_indices.iter())
        {
            let serialized_expected_token_data = token_data.try_to_vec().unwrap();
            let change_data_struct = CompressedAccountData {
                discriminator: TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
                data: serialized_expected_token_data.clone(),
                data_hash: token_data.hash::<Poseidon>().unwrap(),
            };
            expected_compressed_output_accounts.push(OutputCompressedAccountWithPackedContext {
                compressed_account: CompressedAccount {
                    owner: crate::ID,
                    lamports: 0,
                    data: Some(change_data_struct),
                    address: None,
                },
                merkle_tree_index: *merkle_tree_index,
            });
        }
        expected_compressed_output_accounts
    }
}
