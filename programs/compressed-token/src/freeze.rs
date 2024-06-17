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

use crate::FreezeInstruction;
use crate::{
    add_token_data_to_input_compressed_accounts, constants::TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
    cpi_execute_compressed_transaction_transfer,
    delegation::get_input_compressed_accounts_with_merkle_context_and_check_signer,
    token_data::AccountState, InputTokenDataWithContext, TokenData,
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
        create_input_and_output_accounts_freeze_or_thaw::<false, true>(
            &inputs,
            &ctx.accounts.authority.key(),
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

    add_token_data_to_input_compressed_accounts(
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
    let hashed_owner = hash_to_bn254_field_size_be(owner.to_bytes().as_slice())
        .unwrap()
        .0;
    let mut cached_hashed_delegates =
        Vec::<(u8, [u8; 32])>::with_capacity(input_token_data_with_context.len());
    Ok(
        for (i, token_data) in input_token_data_with_context.iter().enumerate() {
            // 83 =
            //      32  mint
            // +    32  owner
            // +    8   amount
            // +    1   delegate
            // +    1   state
            // +    8   delegated_amount
            let mut token_data_bytes = Vec::with_capacity(83);
            let (_hashed_delegate, delegate) = match token_data.delegate_index {
                Some(index) => {
                    let result = cached_hashed_delegates.iter().find(|x| x.0 == index);
                    match result {
                        None => {
                            let delegate = remaining_accounts[index as usize].key();
                            let hashed_delegate =
                                hash_to_bn254_field_size_be(delegate.to_bytes().as_slice())
                                    .unwrap()
                                    .0;
                            cached_hashed_delegates.push((index, hashed_delegate));
                            (Some(hashed_delegate), Some(delegate))
                        }
                        Some((_, hashed_delegate)) => (
                            Some(*hashed_delegate),
                            Some(remaining_accounts[index as usize].key()),
                        ),
                    }
                }
                None => (None, None),
            };
            let state = if IS_FROZEN {
                AccountState::Frozen
            } else {
                AccountState::Initialized
            };
            // 1,000 CU token data and serialize
            let token_data = TokenData {
                mint: mint.clone(),
                owner: *owner,
                amount: token_data.amount,
                delegate,
                state,
                is_native: None,
            };
            token_data.serialize(&mut token_data_bytes).unwrap();
            // TODO: add hash function with hashed inputs
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
        },
    )
}
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedTokenInstructionDataThaw {
    pub proof: CompressedProof,
    pub owner: Pubkey,
    pub input_token_data_with_context: Vec<InputTokenDataWithContext>,
    pub cpi_context: Option<CompressedCpiContext>,
    pub outputs_merkle_tree_index: u8,
}

// pub fn create_input_and_output_accounts_thaw(
//     inputs: &CompressedTokenInstructionDataThaw,
//     mint: &Pubkey,
//     remaining_accounts: &[AccountInfo<'_>],
// ) -> Result<(
//     Vec<PackedCompressedAccountWithMerkleContext>,
//     Vec<OutputCompressedAccountWithPackedContext>,
// )> {
//     let (mut compressed_input_accounts, input_token_data) =
//         get_input_compressed_accounts_with_merkle_context_and_check_signer::<true>(
//             &inputs.owner,
//             remaining_accounts,
//             &inputs.input_token_data_with_context,
//             mint,
//         )?;
//     let mut output_compressed_accounts =
//         vec![OutputCompressedAccountWithPackedContext::default(); compressed_input_accounts.len()];
//     let hashed_mint = hash_to_bn254_field_size_be(&mint.to_bytes()).unwrap().0;
//     create_token_output_accounts::<false>(
//         inputs.input_token_data_with_context.as_slice(),
//         remaining_accounts,
//         mint,
//         &inputs.owner,
//         &inputs.outputs_merkle_tree_index,
//         &mut output_compressed_accounts,
//     )?;
//     add_token_data_to_input_compressed_accounts(
//         &mut compressed_input_accounts,
//         input_token_data.as_slice(),
//         &hashed_mint,
//     )?;
//     Ok((compressed_input_accounts, output_compressed_accounts))
// }

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
