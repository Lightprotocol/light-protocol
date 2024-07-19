use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::{compressed_account::PackedCompressedAccountWithMerkleContext, CompressedCpiContext},
    OutputCompressedAccountWithPackedContext,
};
use light_utils::hash_to_bn254_field_size_be;

use crate::{
    constants::NOT_FROZEN,
    process_transfer::{
        add_token_data_to_input_compressed_accounts, cpi_execute_compressed_transaction_transfer,
        create_output_compressed_accounts, get_cpi_signer_seeds,
        get_input_compressed_accounts_with_merkle_context_and_check_signer, DelegatedTransfer,
        InputTokenDataWithContext,
    },
    BurnInstruction, ErrorCode,
};

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedTokenInstructionDataBurn {
    pub proof: CompressedProof,
    pub input_token_data_with_context: Vec<InputTokenDataWithContext>,
    pub cpi_context: Option<CompressedCpiContext>,
    pub burn_amount: u64,
    pub change_account_merkle_tree_index: u8,
    pub delegated_transfer: Option<DelegatedTransfer>,
}

pub fn process_burn<'a, 'b, 'c, 'info: 'b + 'c>(
    ctx: Context<'a, 'b, 'c, 'info, BurnInstruction<'info>>,
    inputs: Vec<u8>,
) -> Result<()> {
    let inputs: CompressedTokenInstructionDataBurn =
        CompressedTokenInstructionDataBurn::deserialize(&mut inputs.as_slice())?;

    burn_spl_from_pool_pda(&ctx, &inputs)?;
    let mint = ctx.accounts.mint.key();
    let (compressed_input_accounts, output_compressed_accounts) =
        create_input_and_output_accounts_burn(
            &inputs,
            &ctx.accounts.authority.key(),
            ctx.remaining_accounts,
            &mint,
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

#[inline(never)]
pub fn burn_spl_from_pool_pda<'info>(
    ctx: &Context<'_, '_, '_, 'info, BurnInstruction<'info>>,
    inputs: &CompressedTokenInstructionDataBurn,
) -> Result<()> {
    let pre_token_balance = ctx.accounts.token_pool_pda.amount;
    let cpi_accounts = anchor_spl::token::Burn {
        mint: ctx.accounts.mint.to_account_info(),
        from: ctx.accounts.token_pool_pda.to_account_info(),
        authority: ctx.accounts.cpi_authority_pda.to_account_info(),
    };
    let signer_seeds = get_cpi_signer_seeds();
    let signer_seeds_ref = &[&signer_seeds[..]];
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer_seeds_ref,
    );

    anchor_spl::token::burn(cpi_ctx, inputs.burn_amount)?;

    let post_token_balance = TokenAccount::try_deserialize(
        &mut &ctx.accounts.token_pool_pda.to_account_info().data.borrow()[..],
    )?
    .amount;
    // Guard against unexpected behavior of the SPL token program.
    if post_token_balance != pre_token_balance - inputs.burn_amount {
        msg!(
            "post_token_balance {} != pre_token_balance {} - burn_amount {}",
            post_token_balance,
            pre_token_balance,
            inputs.burn_amount
        );
        return err!(crate::ErrorCode::SplTokenSupplyMismatch);
    }
    Ok(())
}

pub fn create_input_and_output_accounts_burn(
    inputs: &CompressedTokenInstructionDataBurn,
    authority: &Pubkey,
    remaining_accounts: &[AccountInfo<'_>],
    mint: &Pubkey,
) -> Result<(
    Vec<PackedCompressedAccountWithMerkleContext>,
    Vec<OutputCompressedAccountWithPackedContext>,
)> {
    let (mut compressed_input_accounts, input_token_data, _) =
        get_input_compressed_accounts_with_merkle_context_and_check_signer::<NOT_FROZEN>(
            authority,
            &inputs.delegated_transfer,
            remaining_accounts,
            &inputs.input_token_data_with_context,
            mint,
        )?;
    let sum_inputs = input_token_data.iter().map(|x| x.amount).sum::<u64>();
    let change_amount = match sum_inputs.checked_sub(inputs.burn_amount) {
        Some(change_amount) => change_amount,
        None => return err!(ErrorCode::ArithmeticUnderflow),
    };
    let sum_lamports = inputs
        .input_token_data_with_context
        .iter()
        .map(|x| x.lamports.unwrap_or(0))
        .sum::<u64>();

    let hashed_mint = match hash_to_bn254_field_size_be(&mint.to_bytes()) {
        Some(hashed_mint) => hashed_mint.0,
        None => return err!(ErrorCode::HashToFieldError),
    };
    let output_compressed_accounts = if change_amount > 0 || sum_lamports > 0 {
        let (is_delegate, authority, delegate) =
            if let Some(delegated_transfer) = inputs.delegated_transfer.as_ref() {
                let mut vec = vec![false; 1];
                if let Some(index) = delegated_transfer.delegate_change_account_index {
                    vec[index as usize] = true;
                } else {
                    return err!(crate::ErrorCode::InvalidDelegateIndex);
                }
                (Some(vec), delegated_transfer.owner, Some(*authority))
            } else {
                (None, *authority, None)
            };
        let mut output_compressed_accounts =
            vec![OutputCompressedAccountWithPackedContext::default(); 1];
        let lamports = if sum_lamports > 0 {
            Some(vec![Some(sum_lamports)])
        } else {
            None
        };
        create_output_compressed_accounts(
            &mut output_compressed_accounts,
            *mint,
            &[authority; 1],
            delegate,
            is_delegate,
            &[change_amount],
            lamports,
            &hashed_mint,
            &[inputs.change_account_merkle_tree_index],
        )?;
        output_compressed_accounts
    } else {
        Vec::new()
    };
    add_token_data_to_input_compressed_accounts::<NOT_FROZEN>(
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
        invoke::processor::CompressedProof,
        sdk::compressed_account::{CompressedAccount, MerkleContext},
    };
    use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

    use crate::{
        get_token_pool_pda,
        process_transfer::{
            get_cpi_authority_pda,
            transfer_sdk::{
                create_input_output_and_remaining_accounts, to_account_metas, TransferSdkError,
            },
            DelegatedTransfer,
        },
        token_data::TokenData,
    };

    use super::CompressedTokenInstructionDataBurn;

    pub struct CreateBurnInstructionInputs {
        pub fee_payer: Pubkey,
        pub authority: Pubkey,
        pub root_indices: Vec<u16>,
        pub proof: CompressedProof,
        pub input_token_data: Vec<TokenData>,
        pub input_compressed_accounts: Vec<CompressedAccount>,
        pub input_merkle_contexts: Vec<MerkleContext>,
        pub change_account_merkle_tree: Pubkey,
        pub mint: Pubkey,
        pub burn_amount: u64,
        pub signer_is_delegate: bool,
    }

    pub fn create_burn_instruction(
        inputs: CreateBurnInstructionInputs,
    ) -> Result<Instruction, TransferSdkError> {
        let (remaining_accounts, input_token_data_with_context, _) =
            create_input_output_and_remaining_accounts(
                &[inputs.change_account_merkle_tree],
                &inputs.input_token_data,
                &inputs.input_compressed_accounts,
                &inputs.input_merkle_contexts,
                &inputs.root_indices,
                &Vec::new(),
            );
        let outputs_merkle_tree_index =
            match remaining_accounts.get(&inputs.change_account_merkle_tree) {
                Some(index) => index,
                None => return Err(TransferSdkError::AccountNotFound),
            };
        let delegated_transfer = if inputs.signer_is_delegate {
            let delegated_transfer = DelegatedTransfer {
                owner: inputs.input_token_data[0].owner,
                delegate_change_account_index: Some(0),
            };
            Some(delegated_transfer)
        } else {
            None
        };
        let inputs_struct = CompressedTokenInstructionDataBurn {
            proof: inputs.proof,
            input_token_data_with_context,
            cpi_context: None,
            change_account_merkle_tree_index: *outputs_merkle_tree_index as u8,
            delegated_transfer,
            burn_amount: inputs.burn_amount,
        };
        let remaining_accounts = to_account_metas(remaining_accounts);
        let mut serialized_ix_data = Vec::new();
        CompressedTokenInstructionDataBurn::serialize(&inputs_struct, &mut serialized_ix_data)
            .map_err(|_| TransferSdkError::SerializationError)?;

        let (cpi_authority_pda, _) = get_cpi_authority_pda();
        let data = crate::instruction::Burn {
            inputs: serialized_ix_data,
        }
        .data();

        let token_pool_pda = get_token_pool_pda(&inputs.mint);

        let accounts = crate::accounts::BurnInstruction {
            fee_payer: inputs.fee_payer,
            authority: inputs.authority,
            cpi_authority_pda,
            mint: inputs.mint,
            token_pool_pda,
            token_program: anchor_spl::token::ID,
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

            data,
        })
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::{
        freeze::test_freeze::{
            create_expected_input_accounts, create_expected_token_output_accounts,
            get_rnd_input_token_data_with_contexts,
        },
        token_data::AccountState,
        TokenData,
    };
    use anchor_lang::solana_program::account_info::AccountInfo;
    use light_system_program::sdk::compressed_account::PackedMerkleContext;
    use rand::Rng;

    // TODO: add randomized and edge case tests
    #[test]
    fn test_burn() {
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
        let test_amounts = vec![0, 1, 10, 100, 1_000, 10_000, 100_000, 1_000_000];
        for test_amount in test_amounts {
            let input_token_data_with_context = vec![InputTokenDataWithContext {
                amount: test_amount,
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    nullifier_queue_pubkey_index: 1,
                    leaf_index: 1,
                    queue_index: None,
                },
                root_index: 0,
                delegate_index: Some(1),
                lamports: None,
            }];
            let inputs = CompressedTokenInstructionDataBurn {
                proof: CompressedProof::default(),
                input_token_data_with_context,
                cpi_context: None,
                burn_amount: std::cmp::min(50, test_amount),
                change_account_merkle_tree_index: 0,
                delegated_transfer: None,
            };
            let (compressed_input_accounts, output_compressed_accounts) =
                create_input_and_output_accounts_burn(
                    &inputs,
                    &authority,
                    &remaining_accounts,
                    &mint,
                )
                .unwrap();
            assert_eq!(compressed_input_accounts.len(), 1);
            let change_amount = test_amount.saturating_sub(inputs.burn_amount);
            assert_eq!(
                output_compressed_accounts.len(),
                std::cmp::min(1, change_amount) as usize
            );
            if change_amount != 0 {
                let expected_change_token_data = TokenData {
                    mint,
                    owner: authority,
                    amount: change_amount,
                    delegate: None,
                    state: AccountState::Initialized,
                };
                let expected_compressed_output_accounts = create_expected_token_output_accounts(
                    vec![expected_change_token_data],
                    vec![0],
                );

                assert_eq!(
                    output_compressed_accounts,
                    expected_compressed_output_accounts
                );
            }
        }
    }

    #[test]
    fn test_rnd_burn() {
        let mut rng = rand::rngs::ThreadRng::default();
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

        let iter = 1_000;
        for _ in 0..iter {
            let authority = Pubkey::new_unique();
            let mint = Pubkey::new_unique();
            let num_inputs = rng.gen_range(1..=8);
            let input_token_data_with_context =
                get_rnd_input_token_data_with_contexts(&mut rng, num_inputs);
            let sum_inputs = input_token_data_with_context
                .iter()
                .map(|x| x.amount)
                .sum::<u64>();
            let burn_amount = rng.gen_range(0..sum_inputs);
            let inputs = CompressedTokenInstructionDataBurn {
                proof: CompressedProof::default(),
                input_token_data_with_context: input_token_data_with_context.clone(),
                cpi_context: None,
                burn_amount,
                change_account_merkle_tree_index: 0,
                delegated_transfer: None,
            };
            let (compressed_input_accounts, output_compressed_accounts) =
                create_input_and_output_accounts_burn(
                    &inputs,
                    &authority,
                    &remaining_accounts,
                    &mint,
                )
                .unwrap();
            let expected_input_accounts = create_expected_input_accounts(
                &input_token_data_with_context,
                &mint,
                &authority,
                remaining_accounts
                    .iter()
                    .map(|x| x.key)
                    .cloned()
                    .collect::<Vec<_>>()
                    .as_slice(),
            );
            assert_eq!(compressed_input_accounts, expected_input_accounts);
            assert_eq!(compressed_input_accounts.len(), num_inputs);
            assert_eq!(output_compressed_accounts.len(), 1);
            let expected_change_token_data = TokenData {
                mint,
                owner: authority,
                amount: sum_inputs - burn_amount,
                delegate: None,
                state: AccountState::Initialized,
            };
            let expected_compressed_output_accounts =
                create_expected_token_output_accounts(vec![expected_change_token_data], vec![0]);

            assert_eq!(
                output_compressed_accounts,
                expected_compressed_output_accounts
            );
        }
    }

    #[test]
    fn failing_tests_burn() {
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
        let input_token_data_with_context = vec![InputTokenDataWithContext {
            amount: 100,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: 0,
                nullifier_queue_pubkey_index: 1,
                leaf_index: 1,
                queue_index: None,
            },
            root_index: 0,
            delegate_index: Some(1),
            lamports: None,
        }];

        // Burn amount too high
        {
            let mut invalid_input_token_data_with_context = input_token_data_with_context.clone();
            invalid_input_token_data_with_context[0].amount = 0;
            let inputs = CompressedTokenInstructionDataBurn {
                proof: CompressedProof::default(),
                input_token_data_with_context: invalid_input_token_data_with_context,
                cpi_context: None,
                burn_amount: 50,
                change_account_merkle_tree_index: 1,
                delegated_transfer: None,
            };
            let result = create_input_and_output_accounts_burn(
                &inputs,
                &authority,
                &remaining_accounts,
                &mint,
            );
            let error_code = ErrorCode::ArithmeticUnderflow as u32 + 6000;
            assert!(matches!(
                result.unwrap_err(),
                anchor_lang::error::Error::AnchorError(error) if error.error_code_number == error_code
            ));
        }
        // invalid authority
        {
            let invalid_authority = Pubkey::new_unique();
            let inputs = CompressedTokenInstructionDataBurn {
                proof: CompressedProof::default(),
                input_token_data_with_context: input_token_data_with_context.clone(),
                cpi_context: None,
                burn_amount: 50,
                change_account_merkle_tree_index: 1,
                delegated_transfer: None,
            };
            let (compressed_input_accounts, output_compressed_accounts) =
                create_input_and_output_accounts_burn(
                    &inputs,
                    &invalid_authority,
                    &remaining_accounts,
                    &mint,
                )
                .unwrap();
            let expected_input_accounts = create_expected_input_accounts(
                &input_token_data_with_context,
                &mint,
                &invalid_authority,
                remaining_accounts
                    .iter()
                    .map(|x| x.key)
                    .cloned()
                    .collect::<Vec<_>>()
                    .as_slice(),
            );
            assert_eq!(compressed_input_accounts, expected_input_accounts);
            let expected_change_token_data = TokenData {
                mint,
                owner: invalid_authority,
                amount: 50,
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
        // Invalid Mint
        {
            let mut invalid_input_token_data_with_context = input_token_data_with_context.clone();
            invalid_input_token_data_with_context[0].amount = 0;
            let invalid_mint = Pubkey::new_unique();
            let inputs = CompressedTokenInstructionDataBurn {
                proof: CompressedProof::default(),
                input_token_data_with_context: input_token_data_with_context.clone(),
                cpi_context: None,
                burn_amount: 50,
                change_account_merkle_tree_index: 1,
                delegated_transfer: None,
            };
            let (compressed_input_accounts, output_compressed_accounts) =
                create_input_and_output_accounts_burn(
                    &inputs,
                    &authority,
                    &remaining_accounts,
                    &invalid_mint,
                )
                .unwrap();
            assert_eq!(compressed_input_accounts.len(), 1);
            assert_eq!(output_compressed_accounts.len(), 1);
            let expected_input_accounts = create_expected_input_accounts(
                &input_token_data_with_context,
                &invalid_mint,
                &authority,
                remaining_accounts
                    .iter()
                    .map(|x| x.key)
                    .cloned()
                    .collect::<Vec<_>>()
                    .as_slice(),
            );
            assert_eq!(compressed_input_accounts, expected_input_accounts);
            let expected_change_token_data = TokenData {
                mint: invalid_mint,
                owner: authority,
                amount: 50,
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
}
