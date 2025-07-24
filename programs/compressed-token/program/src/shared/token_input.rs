use anchor_compressed_token::token_data::TokenData;
use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::checks::check_signer;
use light_compressed_account::instruction_data::with_readonly::ZInAccountMut;
use pinocchio::account_info::AccountInfo;

use crate::constants::TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR;
use light_ctoken_types::context::TokenContext;
use light_ctoken_types::instructions::multi_transfer::ZMultiInputTokenDataWithContext;

/// Creates an input compressed account using zero-copy patterns and index-based account lookup.
///
/// Validates signer authorization (owner or delegate), populates the zero-copy account structure,
/// and computes the appropriate token data hash based on frozen state.
pub fn set_input_compressed_account<const IS_FROZEN: bool>(
    input_compressed_account: &mut ZInAccountMut,
    context: &mut TokenContext,
    input_token_data: &ZMultiInputTokenDataWithContext,
    accounts: &[AccountInfo],
    lamports: u64,
) -> std::result::Result<(), ProgramError> {
    // Get owner from remaining accounts using the owner index
    let owner_account = &accounts[input_token_data.owner as usize];

    // Verify signer authorization using light-account-checks
    let hashed_delegate = if input_token_data.with_delegate() {
        // If delegate is used, delegate must be signer
        let delegate_account = &accounts[input_token_data.delegate as usize];

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

    // Compute data hash using TokenContext for caching
    let hashed_owner = context.get_or_hash_pubkey(&owner_account.key());

    // Get mint hash from context
    let mint_account = &accounts[input_token_data.mint as usize];
    let hashed_mint = context.get_or_hash_mint(mint_account.key())?;

    let mut amount_bytes = [0u8; 32];
    amount_bytes[24..].copy_from_slice(input_token_data.amount.get().to_be_bytes().as_slice());

    // Use appropriate hash function based on frozen state
    let data_hash = if !IS_FROZEN {
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

    input_compressed_account
        .set(
            TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
            data_hash,
            &input_token_data.merkle_context,
            *input_token_data.root_index,
            lamports,
            None, // Token accounts don't have addresses
        )
        .map_err(ProgramError::from)?;

    Ok(())
}
