use anchor_compressed_token::TokenData;
use anchor_lang::solana_program::program_error::ProgramError;
use borsh::BorshSerialize;
use light_compressed_account::instruction_data::with_readonly::ZInAccountMut;
use light_ctoken_types::{
    hash_cache::HashCache, instructions::transfer2::ZMultiInputTokenDataWithContext,
    state::TokenDataVersion,
};
use light_hasher::{sha256::Sha256BE, Hasher};
use light_profiler::profile;
use pinocchio::account_info::AccountInfo;

use crate::shared::owner_validation::verify_owner_or_delegate_signer;

#[inline(always)]
pub fn set_input_compressed_account(
    input_compressed_account: &mut ZInAccountMut,
    hash_cache: &mut HashCache,
    input_token_data: &ZMultiInputTokenDataWithContext,
    accounts: &[AccountInfo],
    lamports: u64,
) -> std::result::Result<(), ProgramError> {
    set_input_compressed_account_inner::<false>(
        input_compressed_account,
        hash_cache,
        input_token_data,
        accounts,
        lamports,
    )
}

#[inline(always)]
pub fn set_input_compressed_account_frozen(
    input_compressed_account: &mut ZInAccountMut,
    hash_cache: &mut HashCache,
    input_token_data: &ZMultiInputTokenDataWithContext,
    accounts: &[AccountInfo],
    lamports: u64,
) -> std::result::Result<(), ProgramError> {
    set_input_compressed_account_inner::<true>(
        input_compressed_account,
        hash_cache,
        input_token_data,
        accounts,
        lamports,
    )
}

/// Creates an input compressed account using zero-copy patterns and index-based account lookup.
///
/// Validates signer authorization (owner or delegate), populates the zero-copy account structure,
/// and computes the appropriate token data hash based on frozen state.
fn set_input_compressed_account_inner<const IS_FROZEN: bool>(
    input_compressed_account: &mut ZInAccountMut,
    hash_cache: &mut HashCache,
    input_token_data: &ZMultiInputTokenDataWithContext,
    accounts: &[AccountInfo],
    lamports: u64,
) -> std::result::Result<(), ProgramError> {
    // Get owner from remaining accounts using the owner index
    let owner_account = accounts
        .get(input_token_data.owner as usize)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    // Verify signer authorization using shared function
    let delegate_account = if input_token_data.has_delegate() {
        Some(
            accounts
                .get(input_token_data.delegate as usize)
                .ok_or(ProgramError::NotEnoughAccountKeys)?,
        )
    } else {
        None
    };

    let verified_delegate = verify_owner_or_delegate_signer(owner_account, delegate_account)?;
    let token_version = TokenDataVersion::try_from(input_token_data.version)?;
    let mint_account = &accounts
        .get(input_token_data.mint as usize)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let data_hash = {
        match token_version {
            TokenDataVersion::ShaFlat => {
                #[profile]
                #[inline(always)]
                fn compute_sha_flat_hash(
                    mint_account: &AccountInfo,
                    owner_account: &AccountInfo,
                    input_token_data: &ZMultiInputTokenDataWithContext,
                    delegate_account: Option<&AccountInfo>,
                ) -> std::result::Result<[u8; 32], ProgramError> {
                    let token_data = TokenData {
                        mint: mint_account.key().into(),
                        owner: owner_account.key().into(),
                        amount: input_token_data.amount.into(),
                        delegate: delegate_account.map(|x| (*x.key()).into()),
                        state: 0, // TODO: double check Initialized state with main
                        tlv: None,
                    };
                    let bytes = token_data
                        .try_to_vec()
                        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;
                    Ok(Sha256BE::hash(bytes.as_slice())?)
                }

                compute_sha_flat_hash(
                    mint_account,
                    owner_account,
                    input_token_data,
                    delegate_account,
                )?
            }
            _ => {
                let hashed_owner = hash_cache.get_or_hash_pubkey(owner_account.key());
                // Get mint hash from hash_cache
                let hashed_mint = hash_cache.get_or_hash_mint(mint_account.key())?;
                let amount_bytes =
                    token_version.serialize_amount_bytes(input_token_data.amount.into())?;

                let hashed_delegate =
                    verified_delegate.map(|delegate| hash_cache.get_or_hash_pubkey(delegate.key()));

                if !IS_FROZEN {
                    TokenData::hash_with_hashed_values(
                        &hashed_mint,
                        &hashed_owner,
                        &amount_bytes,
                        &hashed_delegate.as_ref(),
                    )
                } else {
                    TokenData::hash_frozen_with_hashed_values(
                        &hashed_mint,
                        &hashed_owner,
                        &amount_bytes,
                        &hashed_delegate.as_ref(),
                    )
                }
            }?,
        }
    };

    input_compressed_account.set_z(
        token_version.discriminator(),
        data_hash,
        &input_token_data.merkle_context,
        *input_token_data.root_index,
        lamports,
        None, // Token accounts don't have addresses
    )?;

    Ok(())
}
