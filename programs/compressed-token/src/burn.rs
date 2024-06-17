use anchor_lang::prelude::*;
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::{compressed_account::PackedCompressedAccountWithMerkleContext, CompressedCpiContext},
    OutputCompressedAccountWithPackedContext,
};
use light_utils::hash_to_bn254_field_size_be;

use crate::{
    add_token_data_to_input_compressed_accounts, cpi_execute_compressed_transaction_transfer,
    create_output_compressed_accounts,
    process_transfer::get_input_compressed_accounts_with_merkle_context_and_check_signer,
    ErrorCode, InputTokenDataWithContext, TransferInstruction,
};
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedTokenInstructionDataBurn {
    pub proof: CompressedProof,
    pub mint: Pubkey,
    pub input_token_data_with_context: Vec<InputTokenDataWithContext>,
    pub cpi_context: Option<CompressedCpiContext>,
    pub burn_amount: u64,
    pub change_account_merkle_tree_index: u8,
}

// TODO: make callable by delegate
pub fn process_burn<'a, 'b, 'c, 'info: 'b + 'c>(
    ctx: Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
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
            &None, // TODO: enable
            remaining_accounts,
            &inputs.input_token_data_with_context,
            &inputs.mint,
        )?;
    let sum_inputs = input_token_data.iter().map(|x| x.amount).sum::<u64>();
    let change_amount = match sum_inputs.checked_sub(inputs.burn_amount) {
        Some(change_amount) => change_amount,
        None => return err!(ErrorCode::ArithmeticUnderflow),
    };
    let mut output_compressed_accounts =
        vec![OutputCompressedAccountWithPackedContext::default(); 1];
    let hashed_mint = hash_to_bn254_field_size_be(&inputs.mint.to_bytes())
        .unwrap()
        .0;
    create_output_compressed_accounts::<false, false>(
        &mut output_compressed_accounts,
        inputs.mint,
        &[*authority; 1],
        None,
        None,
        &[change_amount],
        None, // TODO: add wrapped sol support
        &hashed_mint,
        &[inputs.change_account_merkle_tree_index],
    )?;
    add_token_data_to_input_compressed_accounts(
        &mut compressed_input_accounts,
        input_token_data.as_slice(),
        &hashed_mint,
    )?;
    Ok((compressed_input_accounts, output_compressed_accounts))
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
