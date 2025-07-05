use anchor_lang::{
    prelude::borsh, solana_program::program_error::ProgramError, AnchorDeserialize, AnchorSerialize,
};
use light_compressed_account::{
    hash_to_bn254_field_size_be,
    instruction_data::with_readonly::ZInstructionDataInvokeCpiWithReadOnlyMut, pubkey::AsPubkey,
    Pubkey,
};
use light_zero_copy::{num_trait::ZeroCopyNumTrait, ZeroCopyMut, ZeroCopyNew};

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

/// Creates output compressed accounts.
/// Steps:
/// 1. Allocate memory for token data.
/// 2. Create, hash and serialize token data.
/// 3. Create compressed account data.
/// 4. Repeat for every pubkey.
#[allow(clippy::too_many_arguments)]
pub fn create_output_compressed_accounts(
    mut cpi_instruction_struct: ZInstructionDataInvokeCpiWithReadOnlyMut<'_>,
    // output_compressed_accounts: &mut [OutputCompressedAccountWithPackedContext],
    mint_pubkey: impl AsPubkey,
    pubkeys: &[impl AsPubkey],
    delegate: Option<Pubkey>,
    is_delegate: Option<Vec<bool>>,
    amounts: &[impl ZeroCopyNumTrait],
    lamports: Option<Vec<Option<impl ZeroCopyNumTrait>>>,
    hashed_mint: &[u8; 32],
    merkle_tree_indices: &[u8],
) -> Result<u64, ProgramError> {
    let mut sum_lamports = 0;
    let hashed_delegate_store = if let Some(delegate) = delegate {
        hash_to_bn254_field_size_be(delegate.to_bytes().as_slice())
    } else {
        [0u8; 32]
    };
    for (i, (owner, amount)) in pubkeys.iter().zip(amounts.iter()).enumerate() {
        let (delegate, hashed_delegate) = if is_delegate
            .as_ref()
            .map(|is_delegate| is_delegate[i])
            .unwrap_or(false)
        {
            (
                delegate.as_ref().map(|delegate_pubkey| *delegate_pubkey),
                Some(&hashed_delegate_store),
            )
        } else {
            (None, None)
        };
        // Create token data config based on delegate presence
        let token_config: <TokenData as ZeroCopyNew>::ZeroCopyConfig = TokenDataConfig {
            delegate: (delegate.is_some(), ()),
            tlv: (false, vec![]),
        };

        // Get compressed account data from CPI struct
        let compressed_account_data = cpi_instruction_struct.output_compressed_accounts[i]
            .compressed_account
            .data
            .as_mut()
            .ok_or(ProgramError::InvalidAccountData)?;

        // Set discriminator
        compressed_account_data.discriminator = TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR;

        // Create TokenData using zero-copy
        let (mut token_data, _) =
            TokenData::new_zero_copy(compressed_account_data.data, token_config)
                .map_err(ProgramError::from)?;

        // Set token data fields directly on zero-copy struct
        token_data.mint = mint_pubkey.to_anchor_pubkey().into();
        token_data.owner = owner.to_anchor_pubkey().into();
        token_data.amount.set((*amount).into());
        if let Some(z_delegate) = token_data.delegate.as_deref_mut() {
            if let Some(delegate_pubkey) = delegate {
                *z_delegate = delegate_pubkey;
            }
        }
        *token_data.state = AccountState::Initialized as u8;

        // Compute data hash using the anchor TokenData hash_with_hashed_values method
        let hashed_owner = hash_to_bn254_field_size_be(owner.to_pubkey_bytes().as_slice());
        let mut amount_bytes = [0u8; 32];
        amount_bytes[24..].copy_from_slice((*amount).to_bytes_be().as_slice());

        *compressed_account_data.data_hash = AnchorTokenData::hash_with_hashed_values(
            hashed_mint,
            &hashed_owner,
            &amount_bytes,
            &hashed_delegate,
        )
        .map_err(ProgramError::from)?;

        // Set other compressed account fields
        cpi_instruction_struct.output_compressed_accounts[i]
            .compressed_account
            .owner = crate::ID.into();

        let lamports_value = lamports
            .as_ref()
            .and_then(|lamports| lamports[i])
            .unwrap_or(0u64.into());
        sum_lamports += lamports_value.into();
        cpi_instruction_struct.output_compressed_accounts[i]
            .compressed_account
            .lamports
            .set(lamports_value.into());

        // Set merkle tree index from parameter
        *cpi_instruction_struct.output_compressed_accounts[i].merkle_tree_index =
            merkle_tree_indices[i];
    }
    Ok(sum_lamports)
}

