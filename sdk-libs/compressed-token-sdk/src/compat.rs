//! Solana-compatible token types using `solana_pubkey::Pubkey`.
//!
//! This module provides convenience types that use standard Solana pubkeys
//! instead of byte arrays for easier integration.

use light_compressed_account::compressed_account::CompressedAccountWithMerkleContext;
use light_sdk::light_hasher::{sha256::Sha256BE, HasherError};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::{AnchorDeserialize, AnchorSerialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorDeserialize, AnchorSerialize, Default)]
#[repr(u8)]
pub enum AccountState {
    #[default]
    Initialized = 0,
    Frozen = 1,
}

impl From<AccountState> for light_token_interface::state::CompressedTokenAccountState {
    fn from(state: AccountState) -> Self {
        match state {
            AccountState::Initialized => {
                light_token_interface::state::CompressedTokenAccountState::Initialized
            }
            AccountState::Frozen => {
                light_token_interface::state::CompressedTokenAccountState::Frozen
            }
        }
    }
}

impl TryFrom<u8> for AccountState {
    type Error = ProgramError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(AccountState::Initialized),
            1 => Ok(AccountState::Frozen),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }
}

/// TokenData using standard Solana pubkeys.
///
/// For zero-copy operations, use `TokenData` from `light_token_interface`.
#[derive(Debug, PartialEq, Eq, AnchorDeserialize, AnchorSerialize, Clone, Default)]
pub struct TokenData {
    /// The mint associated with this account
    pub mint: Pubkey,
    /// The owner of this account
    pub owner: Pubkey,
    /// The amount of tokens this account holds
    pub amount: u64,
    /// Optional delegate authorized to transfer tokens
    pub delegate: Option<Pubkey>,
    /// The account's state
    pub state: AccountState,
    /// TLV extensions for compressed token accounts
    pub tlv: Option<Vec<light_token_interface::state::ExtensionStruct>>,
}

impl TokenData {
    /// TokenDataVersion 3
    /// CompressedAccount Discriminator `[0,0,0,0,0,0,0,4]`
    #[inline(always)]
    pub fn hash_sha_flat(&self) -> Result<[u8; 32], HasherError> {
        use light_sdk::light_hasher::Hasher;
        let bytes = self.try_to_vec().map_err(|_| HasherError::BorshError)?;
        Sha256BE::hash(bytes.as_slice())
    }
}

/// TokenData with merkle context for verification
#[derive(Debug, Clone, PartialEq)]
pub struct TokenDataWithMerkleContext {
    pub token_data: TokenData,
    pub compressed_account: CompressedAccountWithMerkleContext,
}

impl TokenDataWithMerkleContext {
    /// Only works for sha flat hash
    pub fn hash(&self) -> Result<[u8; 32], HasherError> {
        if let Some(data) = self.compressed_account.compressed_account.data.as_ref() {
            match data.discriminator {
                [0, 0, 0, 0, 0, 0, 0, 4] => self.token_data.hash_sha_flat(),
                _ => Err(HasherError::EmptyInput),
            }
        } else {
            Err(HasherError::EmptyInput)
        }
    }
}

impl From<TokenData> for light_token_interface::state::TokenData {
    fn from(data: TokenData) -> Self {
        use light_token_interface::state::CompressedTokenAccountState;

        Self {
            mint: data.mint.to_bytes().into(),
            owner: data.owner.to_bytes().into(),
            amount: data.amount,
            delegate: data.delegate.map(|d| d.to_bytes().into()),
            state: match data.state {
                AccountState::Initialized => CompressedTokenAccountState::Initialized as u8,
                AccountState::Frozen => CompressedTokenAccountState::Frozen as u8,
            },
            tlv: data.tlv,
        }
    }
}

impl From<light_token_interface::state::TokenData> for TokenData {
    fn from(data: light_token_interface::state::TokenData) -> Self {
        Self {
            mint: Pubkey::new_from_array(data.mint.to_bytes()),
            owner: Pubkey::new_from_array(data.owner.to_bytes()),
            amount: data.amount,
            delegate: data.delegate.map(|d| Pubkey::new_from_array(d.to_bytes())),
            state: AccountState::try_from(data.state).unwrap_or(AccountState::Initialized),
            tlv: data.tlv,
        }
    }
}

/// Type alias for backward compatibility
pub type InputTokenDataCompressible =
    light_token_interface::instructions::transfer2::MultiTokenTransferOutputData;
