use account_compression::StateMerkleTreeAccount;
use anchor_lang::prelude::*;
use light_compressed_account::{
    compressed_account::{CompressedAccount, CompressedAccountData},
    hash_to_bn254_field_size_be,
    instruction_data::{
        compressed_proof::CompressedProof, cpi_context::CompressedCpiContext,
        data::OutputCompressedAccountWithPackedContext, with_readonly::InAccount,
    },
};
use light_ctoken_types::state::CompressedTokenAccountState;

use crate::{
    process_transfer::{
        add_data_hash_to_input_compressed_accounts, cpi_execute_compressed_transaction_transfer,
        get_input_compressed_accounts_with_merkle_context_and_check_signer,
        get_token_account_discriminator, InputTokenDataWithContext, BATCHED_DISCRIMINATOR,
    },
    FreezeInstruction, TokenData,
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
    // CPI context check not needed: freeze/thaw operations don't modify Solana account state
    let (compressed_input_accounts, output_compressed_accounts) =
        create_input_and_output_accounts_freeze_or_thaw::<FROZEN_INPUTS, FROZEN_OUTPUTS>(
            &inputs,
            &ctx.accounts.mint.key(),
            ctx.remaining_accounts,
        )?;
    // TODO: discuss
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

pub fn create_input_and_output_accounts_freeze_or_thaw<
    const FROZEN_INPUTS: bool,
    const FROZEN_OUTPUTS: bool,
>(
    inputs: &CompressedTokenInstructionDataFreeze,
    mint: &Pubkey,
    remaining_accounts: &[AccountInfo<'_>],
) -> Result<(
    Vec<InAccount>,
    Vec<OutputCompressedAccountWithPackedContext>,
)> {
    if inputs.input_token_data_with_context.is_empty() {
        return err!(crate::ErrorCode::NoInputTokenAccountsProvided);
    }
    let (mut compressed_input_accounts, input_token_data, _) =
        get_input_compressed_accounts_with_merkle_context_and_check_signer::<FROZEN_INPUTS>(
            // The signer in this case is the freeze authority. The owner is not
            // required to sign for this instruction. Hence, we pass the owner
            // from a variable instead of an account to still reproduce value
            // token data hashes for the input accounts.
            &inputs.owner,
            &None,
            remaining_accounts,
            &inputs.input_token_data_with_context,
            mint,
        )?;
    let output_len = compressed_input_accounts.len();
    let mut output_compressed_accounts =
        vec![OutputCompressedAccountWithPackedContext::default(); output_len];
    let hashed_mint = hash_to_bn254_field_size_be(mint.to_bytes().as_slice());
    create_token_output_accounts::<FROZEN_OUTPUTS>(
        inputs.input_token_data_with_context.as_slice(),
        remaining_accounts,
        mint,
        // The signer in this case is the freeze authority. The owner is not
        // required to sign for this instruction. Hence, we pass the owner
        // from a variable instead of an account to still reproduce value
        // token data hashes for the input accounts.
        &inputs.owner,
        &inputs.outputs_merkle_tree_index,
        &mut output_compressed_accounts,
    )?;

    add_data_hash_to_input_compressed_accounts::<FROZEN_INPUTS>(
        &mut compressed_input_accounts,
        input_token_data.as_slice(),
        &hashed_mint,
        remaining_accounts,
    )?;
    Ok((compressed_input_accounts, output_compressed_accounts))
}

/// This is a separate function from create_output_compressed_accounts to allow
/// for a flexible number of delegates. create_output_compressed_accounts only
/// supports one delegate.
fn create_token_output_accounts<const IS_FROZEN: bool>(
    input_token_data_with_context: &[InputTokenDataWithContext],
    remaining_accounts: &[AccountInfo],
    mint: &Pubkey,
    owner: &Pubkey,
    outputs_merkle_tree_index: &u8,
    output_compressed_accounts: &mut [OutputCompressedAccountWithPackedContext],
) -> Result<()> {
    for (i, token_data_with_context) in input_token_data_with_context.iter().enumerate() {
        // 106/74 =
        //      32      mint
        // +    32      owner
        // +    8       amount
        // +    1 + 32  option + delegate (optional)
        // +    1       state
        // +    1       tlv
        let capacity = if token_data_with_context.delegate_index.is_some() {
            107
        } else {
            75
        };
        let mut token_data_bytes = Vec::with_capacity(capacity);
        let delegate = token_data_with_context
            .delegate_index
            .map(|index| remaining_accounts[index as usize].key());
        let state = if IS_FROZEN {
            CompressedTokenAccountState::Frozen as u8
        } else {
            CompressedTokenAccountState::Initialized as u8
        };
        // 1,000 CU token data and serialize
        let token_data = TokenData {
            mint: (*mint).into(),
            owner: (*owner).into(),
            amount: token_data_with_context.amount,
            delegate: delegate.map(|k| k.into()),
            state,
            tlv: None,
        };
        token_data.serialize(&mut token_data_bytes)?;

        let discriminator_bytes = &remaining_accounts[token_data_with_context
            .merkle_context
            .merkle_tree_pubkey_index
            as usize]
            .try_borrow_data()?[0..8];
        use anchor_lang::Discriminator;
        let data_hash = match discriminator_bytes {
            StateMerkleTreeAccount::DISCRIMINATOR => token_data.hash_v1(),
            BATCHED_DISCRIMINATOR => token_data.hash_v2(),
            _ => panic!(), // TODO: throw error
        }
        .map_err(ProgramError::from)?;

        let discriminator = get_token_account_discriminator(discriminator_bytes)?;

        let data: CompressedAccountData = CompressedAccountData {
            discriminator,
            data: token_data_bytes,
            data_hash,
        };
        output_compressed_accounts[i] = OutputCompressedAccountWithPackedContext {
            compressed_account: CompressedAccount {
                owner: crate::ID.into(),
                lamports: token_data_with_context.lamports.unwrap_or(0),
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
    use light_compressed_account::{
        compressed_account::{CompressedAccount, MerkleContext},
        instruction_data::compressed_proof::CompressedProof,
    };
    use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

    use super::CompressedTokenInstructionDataFreeze;
    use crate::{
        process_transfer::transfer_sdk::{
            create_input_output_and_remaining_accounts, to_account_metas, TransferSdkError,
        },
        TokenData,
    };

    pub struct CreateInstructionInputs {
        pub fee_payer: Pubkey,
        pub authority: Pubkey,
        pub root_indices: Vec<Option<u16>>,
        pub proof: CompressedProof,
        pub input_token_data: Vec<TokenData>,
        pub input_compressed_accounts: Vec<CompressedAccount>,
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
                &inputs.input_compressed_accounts,
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
            owner: inputs.input_token_data[0].owner.into(),
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
            mint: inputs.input_token_data[0].mint.into(),
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
    use account_compression::StateMerkleTreeAccount;
    use anchor_lang::{solana_program::account_info::AccountInfo, Discriminator};
    use light_compressed_account::{compressed_account::PackedMerkleContext, Pubkey};
    use light_ctoken_types::state::CompressedTokenAccountState;
    use rand::Rng;

    use super::*;
    use crate::{constants::TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR, TokenData};

    // TODO: add randomized and edge case tests
    #[test]
    fn test_freeze() {
        let merkle_tree_pubkey = anchor_lang::prelude::Pubkey::new_unique();
        let mut merkle_tree_account_lamports = 0;
        let mut merkle_tree_account_data = StateMerkleTreeAccount::DISCRIMINATOR.to_vec();
        let nullifier_queue_pubkey = anchor_lang::prelude::Pubkey::new_unique();
        let mut nullifier_queue_account_lamports = 0;
        let mut nullifier_queue_account_data = Vec::new();
        let delegate = anchor_lang::prelude::Pubkey::new_unique();
        let mut delegate_account_lamports = 0;
        let mut delegate_account_data = Vec::new();
        let merkle_tree_pubkey_1 = anchor_lang::prelude::Pubkey::new_unique();
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
                &delegate,
                false,
                false,
                &mut delegate_account_lamports,
                &mut delegate_account_data,
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
        let owner = Pubkey::new_unique();
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
                delegate_index: None,
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
                delegate_index: Some(2),
                lamports: None,
                tlv: None,
            },
        ];
        // Freeze
        {
            let inputs = CompressedTokenInstructionDataFreeze {
                proof: CompressedProof::default(),
                owner: owner.into(),
                input_token_data_with_context: input_token_data_with_context.clone(),
                cpi_context: None,
                outputs_merkle_tree_index: 3,
            };
            let (compressed_input_accounts, output_compressed_accounts) =
                create_input_and_output_accounts_freeze_or_thaw::<false, true>(
                    &inputs,
                    &mint.into(),
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
                state: CompressedTokenAccountState::Frozen as u8,
                tlv: None,
            };
            let expected_delegated_token_data = TokenData {
                mint,
                owner,
                amount: 101,
                delegate: Some(delegate.into()),
                state: CompressedTokenAccountState::Frozen as u8,
                tlv: None,
            };

            let expected_compressed_output_accounts = create_expected_token_output_accounts(
                vec![expected_change_token_data, expected_delegated_token_data],
                vec![3u8; 2],
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
                owner: owner.into(),
                input_token_data_with_context,
                cpi_context: None,
                outputs_merkle_tree_index: 3,
            };
            let (compressed_input_accounts, output_compressed_accounts) =
                create_input_and_output_accounts_freeze_or_thaw::<true, false>(
                    &inputs,
                    &mint.into(),
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
                state: CompressedTokenAccountState::Initialized as u8,
                tlv: None,
            };
            let expected_delegated_token_data = TokenData {
                mint,
                owner,
                amount: 101,
                delegate: Some(delegate.into()),
                state: CompressedTokenAccountState::Initialized as u8,
                tlv: None,
            };

            let expected_compressed_output_accounts = create_expected_token_output_accounts(
                vec![expected_change_token_data, expected_delegated_token_data],
                vec![3u8; 2],
            );
            assert_eq!(
                output_compressed_accounts,
                expected_compressed_output_accounts
            );
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
                data_hash: token_data.hash_v1().unwrap(),
            };
            expected_compressed_output_accounts.push(OutputCompressedAccountWithPackedContext {
                compressed_account: CompressedAccount {
                    owner: crate::ID.into(),
                    lamports: 0,
                    data: Some(change_data_struct),
                    address: None,
                },
                merkle_tree_index: *merkle_tree_index,
            });
        }
        expected_compressed_output_accounts
    }
    pub fn get_rnd_input_token_data_with_contexts(
        rng: &mut rand::rngs::ThreadRng,
        num: usize,
    ) -> Vec<InputTokenDataWithContext> {
        let mut vec = Vec::with_capacity(num);
        for _ in 0..num {
            let delegate_index = if rng.gen_bool(0.5) { Some(1) } else { None };
            vec.push(InputTokenDataWithContext {
                amount: rng.gen_range(0..1_000_000_000),
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    queue_pubkey_index: 1,
                    leaf_index: rng.gen_range(0..1_000_000_000),
                    prove_by_index: false,
                },
                root_index: rng.gen_range(0..=65_535),
                delegate_index,
                lamports: None,
                tlv: None,
            });
        }
        vec
    }
    pub fn create_expected_input_accounts(
        input_token_data_with_context: &[InputTokenDataWithContext],
        mint: &anchor_lang::prelude::Pubkey,
        owner: &anchor_lang::prelude::Pubkey,
        remaining_accounts: &[anchor_lang::prelude::Pubkey],
    ) -> Vec<InAccount> {
        input_token_data_with_context
            .iter()
            .map(|x| {
                let delegate = x
                    .delegate_index
                    .map(|index| remaining_accounts[index as usize]);
                let token_data = TokenData {
                    mint: mint.into(),
                    owner: owner.into(),
                    amount: x.amount,
                    delegate: delegate.map(|d| d.into()),
                    state: CompressedTokenAccountState::Initialized as u8,
                    tlv: None,
                };
                let mut data = Vec::new();
                token_data.serialize(&mut data).unwrap();
                let data_hash = token_data.hash_v1().unwrap();
                InAccount {
                    lamports: 0,
                    address: None,
                    data_hash,
                    discriminator: TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
                    root_index: x.root_index,
                    merkle_context: x.merkle_context,
                }
            })
            .collect()
    }
}
