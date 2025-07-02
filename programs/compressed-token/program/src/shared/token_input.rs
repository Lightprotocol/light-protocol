use anchor_compressed_token::TokenData;
use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::instruction_data::with_readonly::ZInAccountMut;
use light_ctoken_types::{
    hash_cache::HashCache,
    instructions::transfer2::{TokenAccountVersion, ZMultiInputTokenDataWithContext},
};
use pinocchio::account_info::AccountInfo;

use crate::shared::owner_validation::verify_owner_or_delegate_signer;

/// Creates an input compressed account using zero-copy patterns and index-based account lookup.
///
/// Validates signer authorization (owner or delegate), populates the zero-copy account structure,
/// and computes the appropriate token data hash based on frozen state.
pub fn set_input_compressed_account<const IS_FROZEN: bool>(
    input_compressed_account: &mut ZInAccountMut,
    hash_cache: &mut HashCache,
    input_token_data: &ZMultiInputTokenDataWithContext,
    accounts: &[AccountInfo],
    lamports: u64,
) -> std::result::Result<(), ProgramError> {
    // Get owner from remaining accounts using the owner index
    let owner_account = &accounts[input_token_data.owner as usize];

    // Verify signer authorization using shared function
    let delegate_account = if input_token_data.with_delegate() {
        Some(&accounts[input_token_data.delegate as usize])
    } else {
        None
    };

    let verified_delegate = verify_owner_or_delegate_signer(owner_account, delegate_account)?;
    let hashed_delegate =
        verified_delegate.map(|delegate| hash_cache.get_or_hash_pubkey(delegate.key()));

    // Compute data hash using HashCache for caching
    let hashed_owner = hash_cache.get_or_hash_pubkey(owner_account.key());

    // Get mint hash from hash_cache
    let mint_account = &accounts[input_token_data.mint as usize];
    let hashed_mint = hash_cache.get_or_hash_mint(mint_account.key())?;

    let version = TokenAccountVersion::try_from(input_token_data.version)?;
    let amount_bytes = version.serialize_amount_bytes(input_token_data.amount.get());

    let data_hash = if !IS_FROZEN {
        TokenData::hash_with_hashed_values(
            &hashed_mint,
            &hashed_owner,
            &amount_bytes,
            &hashed_delegate.as_ref(),
        )?
    } else {
        TokenData::hash_frozen_with_hashed_values(
            &hashed_mint,
            &hashed_owner,
            &amount_bytes,
            &hashed_delegate.as_ref(),
        )?
    };

    input_compressed_account.set_z(
        version.discriminator(),
        data_hash,
        &input_token_data.merkle_context,
        *input_token_data.root_index,
        lamports,
        None, // Token accounts don't have addresses
    )?;

    Ok(())
}
