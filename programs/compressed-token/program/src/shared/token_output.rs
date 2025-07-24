// Import the anchor TokenData for hash computation
use anchor_compressed_token::{token_data::TokenData as AnchorTokenData, ErrorCode};
use anchor_lang::{
    prelude::{borsh, ProgramError},
    AnchorDeserialize, AnchorSerialize,
};
use light_compressed_account::{
    instruction_data::data::ZOutputCompressedAccountWithPackedContextMut, Pubkey,
};
use light_zero_copy::{num_trait::ZeroCopyNumTrait, ZeroCopyMut, ZeroCopyNew};

use crate::constants::TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR;
use light_ctoken_types::context::TokenContext;

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

// Implementation for zero-copy mutable TokenData
impl<'a> ZTokenDataMut<'a> {
    /// Set all fields of the TokenData struct at once
    #[inline]
    pub fn set(
        &mut self,
        mint: Pubkey,
        owner: Pubkey,
        amount: impl ZeroCopyNumTrait,
        delegate: Option<Pubkey>,
        state: AccountState,
    ) -> Result<(), ErrorCode> {
        self.mint = mint;
        self.owner = owner;
        self.amount.set(amount.into());
        if let Some(z_delegate) = self.delegate.as_deref_mut() {
            *z_delegate = delegate.ok_or(ErrorCode::InstructionDataExpectedDelegate)?;
        }
        if self.delegate.is_none() && delegate.is_some() {
            return Err(ErrorCode::ZeroCopyExpectedDelegate);
        }
        *self.state = state as u8;

        if self.tlv.is_some() {
            return Err(ErrorCode::TokenDataTlvUnimplemented);
        }
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn set_output_compressed_account(
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
    // Create TokenData using zero-copy to compute the data hash
    let data_hash = {
        // Get compressed account data from CPI struct to temporarily create TokenData
        let compressed_account_data = output_compressed_account
            .compressed_account
            .data
            .as_mut()
            .ok_or(ProgramError::InvalidAccountData)?;

        // Create token data config based on delegate presence
        let token_config: <TokenData as ZeroCopyNew>::ZeroCopyConfig = TokenDataConfig {
            delegate: (delegate.is_some(), ()),
            tlv: (false, vec![]),
        };

        let (mut token_data, _) =
            TokenData::new_zero_copy(compressed_account_data.data, token_config)
                .map_err(ProgramError::from)?;

        // Convert ErrorCode to ProgramError explicitly
        token_data
            .set(
                mint_pubkey,
                owner,
                amount,
                delegate,
                AccountState::Initialized,
            )
            .map_err(|e| ProgramError::Custom(e.into()))?;

        // Compute data hash using the anchor TokenData hash_with_hashed_values method
        let hashed_owner = context.get_or_hash_pubkey(&owner.into());
        let mut amount_bytes = [0u8; 32];
        amount_bytes[24..].copy_from_slice(amount.to_bytes_be().as_slice());

        let hashed_delegate =
            delegate.map(|delegate_pubkey| context.get_or_hash_pubkey(&delegate_pubkey.into()));

        AnchorTokenData::hash_with_hashed_values(
            hashed_mint,
            &hashed_owner,
            &amount_bytes,
            &hashed_delegate.as_ref(),
        )
        .map_err(ProgramError::from)?
    };

    let lamports_value = lamports.unwrap_or(0u64.into()).into();
    output_compressed_account
        .set(
            crate::ID.into(),
            lamports_value,
            None, // Token accounts don't have addresses
            merkle_tree_index,
            TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
            data_hash,
        )
        .map_err(ProgramError::from)?;

    Ok(())
}
