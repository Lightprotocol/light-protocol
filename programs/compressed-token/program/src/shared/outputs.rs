use anchor_lang::{
    prelude::borsh, solana_program::program_error::ProgramError, AnchorDeserialize, AnchorSerialize,
};
use light_compressed_account::{
    instruction_data::data::ZOutputCompressedAccountWithPackedContextMut, Pubkey,
};
use light_zero_copy::{num_trait::ZeroCopyNumTrait, ZeroCopyMut, ZeroCopyNew};

use super::context::TokenContext;

use crate::constants::TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR;

// Import the anchor TokenData for hash computation
use anchor_compressed_token::token_data::TokenData as AnchorTokenData;

#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum AccountState {
    Initialized,
    Frozen,
}

#[derive(Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, Clone, ZeroCopyMut)]
pub struct TokenData {
    /// The mint associated with this account
    pub mint: Pubkey,
    /// The owner of this account.
    pub owner: Pubkey,
    /// The amount of tokens this account holds.
    pub amount: u64,
    /// If `delegate` is `Some` then `delegated_amount` represents
    /// the amount authorized by the delegate
    pub delegate: Option<Pubkey>,
    /// The account's state (u8: 0 = Initialized, 1 = Frozen)
    pub state: u8,
    /// Placeholder for TokenExtension tlv data (unimplemented)
    pub tlv: Option<Vec<u8>>,
}

#[allow(clippy::too_many_arguments)]
pub fn create_output_compressed_account(
    output_compressed_account: &mut ZOutputCompressedAccountWithPackedContextMut<'_>,
    context: &mut TokenContext,
    owner: Pubkey,
    delegate: Option<Pubkey>,
    amount: impl ZeroCopyNumTrait,
    lamports: Option<impl ZeroCopyNumTrait>,
    mint_pubkey: Pubkey,
    hashed_mint: &[u8; 32],
    merkle_tree_index: u8,
) -> Result<(), ProgramError> {
    // Get compressed account data from CPI struct
    let compressed_account_data = output_compressed_account
        .compressed_account
        .data
        .as_mut()
        .ok_or(ProgramError::InvalidAccountData)?;

    // Set discriminator
    compressed_account_data.discriminator = TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR;
    // Create TokenData using zero-copy
    {
        // Create token data config based on delegate presence
        let token_config: <TokenData as ZeroCopyNew>::ZeroCopyConfig = TokenDataConfig {
            delegate: (delegate.is_some(), ()),
            tlv: (false, vec![]),
        };

        let (mut token_data, _) =
            TokenData::new_zero_copy(compressed_account_data.data, token_config)
                .map_err(ProgramError::from)?;

        // Set token data fields directly on zero-copy struct
        token_data.mint = mint_pubkey;
        token_data.owner = owner;
        token_data.amount.set(amount.into());
        if let Some(z_delegate) = token_data.delegate.as_deref_mut() {
            let delegate_pubkey = delegate.ok_or(ProgramError::InvalidAccountData)?;
            *z_delegate = delegate_pubkey;
        }
        *token_data.state = AccountState::Initialized as u8;
    }
    // Compute data hash using the anchor TokenData hash_with_hashed_values method
    {
        let hashed_owner = context.get_or_hash_pubkey(&owner);
        let mut amount_bytes = [0u8; 32];
        amount_bytes[24..].copy_from_slice(amount.to_bytes_be().as_slice());

        let hashed_delegate =
            delegate.map(|delegate_pubkey| context.get_or_hash_pubkey(&delegate_pubkey));

        let hash_result = AnchorTokenData::hash_with_hashed_values(
            hashed_mint,
            &hashed_owner,
            &amount_bytes,
            &hashed_delegate.as_ref(),
        )
        .map_err(ProgramError::from)?;
        compressed_account_data
            .data_hash
            .copy_from_slice(&hash_result);
    }

    // Set other compressed account fields
    {
        output_compressed_account.compressed_account.owner = crate::ID.into();

        let lamports_value = lamports.unwrap_or(0u64.into());
        output_compressed_account
            .compressed_account
            .lamports
            .set(lamports_value.into());

        // Set merkle tree index from parameter
        *output_compressed_account.merkle_tree_index = merkle_tree_index;
    }

    Ok(())
}
