use anchor_lang::prelude::*;
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::{compressed_account::PackedCompressedAccountWithMerkleContext, CompressedCpiContext},
    OutputCompressedAccountWithPackedContext,
};
use light_utils::hash_to_bn254_field_size_be;

use crate::{
    create_output_compressed_accounts,
    process_transfer::{
        add_token_data_to_input_compressed_accounts, cpi_execute_compressed_transaction_transfer,
        get_input_compressed_accounts_with_merkle_context_and_check_signer, DelegatedTransfer,
        InputTokenDataWithContext,
    },
    ErrorCode, GenericInstruction,
};

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedTokenInstructionDataBurn {
    pub proof: CompressedProof,
    pub mint: Pubkey,
    pub input_token_data_with_context: Vec<InputTokenDataWithContext>,
    pub cpi_context: Option<CompressedCpiContext>,
    pub burn_amount: u64,
    pub change_account_merkle_tree_index: u8,
    pub delegated_transfer: Option<DelegatedTransfer>,
}

pub fn process_burn<'a, 'b, 'c, 'info: 'b + 'c>(
    ctx: Context<'a, 'b, 'c, 'info, GenericInstruction<'info>>,
    inputs: Vec<u8>,
) -> Result<()> {
    let inputs: CompressedTokenInstructionDataBurn =
        CompressedTokenInstructionDataBurn::deserialize(&mut inputs.as_slice())?;
    let (compressed_input_accounts, output_compressed_accounts) =
        create_input_and_output_accounts_burn(
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

pub fn create_input_and_output_accounts_burn(
    inputs: &CompressedTokenInstructionDataBurn,
    authority: &Pubkey,
    remaining_accounts: &[AccountInfo<'_>],
) -> Result<(
    Vec<PackedCompressedAccountWithMerkleContext>,
    Vec<OutputCompressedAccountWithPackedContext>,
)> {
    let (mut compressed_input_accounts, input_token_data) =
        get_input_compressed_accounts_with_merkle_context_and_check_signer::<false>(
            authority,
            &inputs.delegated_transfer,
            remaining_accounts,
            &inputs.input_token_data_with_context,
            &inputs.mint,
        )?;
    let sum_inputs = input_token_data.iter().map(|x| x.amount).sum::<u64>();
    let change_amount = match sum_inputs.checked_sub(inputs.burn_amount) {
        Some(change_amount) => change_amount,
        None => return err!(ErrorCode::ArithmeticUnderflow),
    };

    let hashed_mint = match hash_to_bn254_field_size_be(&inputs.mint.to_bytes()) {
        Some(hashed_mint) => hashed_mint.0,
        None => return err!(ErrorCode::HashToFieldError),
    };
    let output_compressed_accounts = if change_amount > 0 {
        let (is_delegate, authority, delegate) =
            if let Some(delegated_transfer) = inputs.delegated_transfer.as_ref() {
                let mut vec = vec![false; 1];
                vec[delegated_transfer.delegate_change_account_index as usize] = true;
                (Some(vec), delegated_transfer.owner, Some(*authority))
            } else {
                (None, *authority, None)
            };
        let mut output_compressed_accounts =
            vec![OutputCompressedAccountWithPackedContext::default(); 1];
        create_output_compressed_accounts::<true, false>(
            &mut output_compressed_accounts,
            inputs.mint,
            &[authority; 1],
            delegate,
            is_delegate,
            &[change_amount],
            None, // TODO: add wrapped sol support
            &hashed_mint,
            &[inputs.change_account_merkle_tree_index],
        )?;
        output_compressed_accounts
    } else {
        Vec::new()
    };
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
                &inputs.input_merkle_contexts,
                &inputs.root_indices,
                &Vec::new(),
            );
        let outputs_merkle_tree_index = remaining_accounts
            .get(&inputs.change_account_merkle_tree)
            .unwrap();
        let delegated_transfer = if inputs.signer_is_delegate {
            let delegated_transfer = DelegatedTransfer {
                owner: inputs.input_token_data[0].owner,
                delegate_change_account_index: 0,
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
            mint: inputs.mint,
            burn_amount: inputs.burn_amount,
        };
        let remaining_accounts = to_account_metas(remaining_accounts);
        let mut serialized_ix_data = Vec::new();
        CompressedTokenInstructionDataBurn::serialize(&inputs_struct, &mut serialized_ix_data)
            .unwrap();

        let (cpi_authority_pda, _) = get_cpi_authority_pda();
        let data = crate::instruction::Burn {
            inputs: serialized_ix_data,
        }
        .data();

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

            data,
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
                delegate_index: Some(1),
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
                delegate_index: None,
            },
        ];
        let inputs = CompressedTokenInstructionDataBurn {
            proof: CompressedProof::default(),
            mint,
            input_token_data_with_context,
            cpi_context: None,
            burn_amount: 50,
            change_account_merkle_tree_index: 1,
            delegated_transfer: None,
        };
        let (compressed_input_accounts, output_compressed_accounts) =
            create_input_and_output_accounts_burn(&inputs, &authority, &remaining_accounts)
                .unwrap();
        assert_eq!(compressed_input_accounts.len(), 2);
        assert_eq!(output_compressed_accounts.len(), 1);
        let expected_change_token_data = TokenData {
            mint,
            owner: authority,
            amount: 151,
            delegate: None,
            state: AccountState::Initialized,
            is_native: None,
        };
        let expected_compressed_output_accounts =
            create_expected_token_output_accounts(vec![expected_change_token_data], vec![1]);

        assert_eq!(
            output_compressed_accounts,
            expected_compressed_output_accounts
        );
    }
}
