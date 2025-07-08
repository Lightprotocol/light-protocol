use anchor_compressed_token::token_data::TokenData;
use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::checks::check_signer;
use light_compressed_account::instruction_data::with_readonly::ZInAccountMut;
use pinocchio::account_info::AccountInfo;

use super::context::TokenContext;
use crate::{
    constants::TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
    multi_transfer::instruction_data::ZMultiInputTokenDataWithContext,
};

/// Creates an input compressed account using zero-copy patterns and index-based account lookup.
///
/// Validates signer authorization (owner or delegate), populates the zero-copy account structure,
/// and computes the appropriate token data hash based on frozen state.
#[allow(clippy::too_many_arguments)]
pub fn create_input_compressed_account<const IS_FROZEN: bool>(
    input_compressed_account: &mut ZInAccountMut,
    context: &mut TokenContext,
    input_token_data: &ZMultiInputTokenDataWithContext,
    remaining_accounts: &[AccountInfo],
    lamports: u64,
) -> std::result::Result<(), ProgramError> {
    // Get owner from remaining accounts using the owner index
    let owner_account = &remaining_accounts[input_token_data.owner as usize];
    let owner = *owner_account.key();

    // Verify signer authorization using light-account-checks
    let hashed_delegate = if input_token_data.with_delegate() {
        // If delegate is used, delegate must be signer
        let delegate_account = &remaining_accounts[input_token_data.delegate as usize];

        check_signer(delegate_account).map_err(|e| {
            anchor_lang::solana_program::msg!(
                "Delegate signer: {:?}",
                solana_pubkey::Pubkey::new_from_array(*delegate_account.key())
            );
            anchor_lang::solana_program::msg!("Delegate signer check failed: {:?}", e);
            ProgramError::from(e)
        })?;
        Some(context.get_or_hash_pubkey(delegate_account.key()))
    } else {
        // If no delegate, owner must be signer

        check_signer(owner_account).map_err(|e| {
            anchor_lang::solana_program::msg!(
                "Checking owner signer: {:?}",
                solana_pubkey::Pubkey::new_from_array(*owner_account.key())
            );
            anchor_lang::solana_program::msg!("Owner signer check failed: {:?}", e);
            ProgramError::from(e)
        })?;
        None
    };

    // Create ZInAccountMut with proper fields
    input_compressed_account.lamports.set(lamports);
    input_compressed_account.discriminator = TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR;
    // Set merkle context fields manually due to mutability constraints
    input_compressed_account
        .merkle_context
        .merkle_tree_pubkey_index = input_token_data.merkle_context.merkle_tree_pubkey_index;
    input_compressed_account.merkle_context.queue_pubkey_index =
        input_token_data.merkle_context.queue_pubkey_index;
    input_compressed_account
        .merkle_context
        .leaf_index
        .set(input_token_data.merkle_context.leaf_index.into());
    input_compressed_account.merkle_context.prove_by_index =
        input_token_data.merkle_context.prove_by_index;
    input_compressed_account
        .root_index
        .set(input_token_data.root_index.get());
    input_compressed_account.address = None;

    // TLV handling is now done separately in the parent instruction data
    // Compute data hash using TokenContext for caching
    let hashed_owner = context.get_or_hash_pubkey(&owner);

    // Get mint hash from context
    let mint_account = &remaining_accounts[input_token_data.mint as usize];
    let hashed_mint = context.get_or_hash_mint(mint_account.key())?;

    let mut amount_bytes = [0u8; 32];
    amount_bytes[24..].copy_from_slice(input_token_data.amount.get().to_be_bytes().as_slice());

    // Use appropriate hash function based on frozen state
    input_compressed_account.data_hash = if !IS_FROZEN {
        TokenData::hash_with_hashed_values(
            &hashed_mint,
            &hashed_owner,
            &amount_bytes,
            &hashed_delegate.as_ref(),
        )
        .map_err(ProgramError::from)?
    } else {
        TokenData::hash_frozen_with_hashed_values(
            &hashed_mint,
            &hashed_owner,
            &amount_bytes,
            &hashed_delegate.as_ref(),
        )
        .map_err(ProgramError::from)?
    };

    Ok(())
}
