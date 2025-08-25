// Import the anchor TokenData for hash computation
use anchor_lang::prelude::ProgramError;
use light_compressed_account::{
    instruction_data::data::ZOutputCompressedAccountWithPackedContextMut, Pubkey,
};
use light_ctoken_types::state::{AccountState, TokenData, TokenDataConfig};
use light_ctoken_types::{hash_cache::HashCache, state::TokenDataVersion};
use light_zero_copy::{num_trait::ZeroCopyNumTrait, ZeroCopyNew};

/// 1. Set token account data
/// 2. Create token account data hash
/// 3. Set output compressed account
#[allow(clippy::too_many_arguments)]
pub fn set_output_compressed_account<const IS_FROZEN: bool>(
    output_compressed_account: &mut ZOutputCompressedAccountWithPackedContextMut<'_>,
    hash_cache: &mut HashCache,
    owner: Pubkey,
    delegate: Option<Pubkey>,
    amount: impl ZeroCopyNumTrait,
    lamports: Option<impl ZeroCopyNumTrait>,
    mint_pubkey: Pubkey,
    hashed_mint: &[u8; 32],
    merkle_tree_index: u8,
    version: u8,
) -> Result<(), ProgramError> {
    // 1. Set token account data
    {
        // Get compressed account data from CPI struct to temporarily create TokenData
        let compressed_account_data = output_compressed_account
            .compressed_account
            .data
            .as_mut()
            .ok_or(ProgramError::InvalidAccountData)?;

        // Create token data config based on delegate presence
        let token_config = TokenDataConfig {
            delegate: (delegate.is_some(), ()),
            tlv: (false, vec![]),
        };

        let (mut token_data, _) =
            TokenData::new_zero_copy(compressed_account_data.data, token_config)
                .map_err(ProgramError::from)?;

        token_data.set(
            mint_pubkey,
            owner,
            amount,
            delegate,
            AccountState::Initialized,
        )?;
    }
    let token_version = TokenDataVersion::try_from(version)?;
    // 2. Create TokenData using zero-copy to compute the data hash
    let data_hash = {
        let hashed_owner = hash_cache.get_or_hash_pubkey(&owner.into());
        let amount_bytes = token_version.serialize_amount_bytes(amount.into());

        let hashed_delegate =
            delegate.map(|delegate_pubkey| hash_cache.get_or_hash_pubkey(&delegate_pubkey.into()));

        if !IS_FROZEN {
            TokenData::hash_with_hashed_values(
                hashed_mint,
                &hashed_owner,
                &amount_bytes,
                &hashed_delegate.as_ref(),
            )
        } else {
            TokenData::hash_frozen_with_hashed_values(
                hashed_mint,
                &hashed_owner,
                &amount_bytes,
                &hashed_delegate.as_ref(),
            )
        }
    }?;
    // 3. Set output compressed account
    let lamports_value = lamports.unwrap_or(0u64.into()).into();
    output_compressed_account.set(
        crate::ID.into(),
        lamports_value,
        None, // Token accounts don't have addresses
        merkle_tree_index,
        token_version.discriminator(),
        data_hash,
    )?;

    Ok(())
}
