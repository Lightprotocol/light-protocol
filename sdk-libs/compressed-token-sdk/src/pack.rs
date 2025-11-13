//! Pack implementation for TokenData types for c-tokens.
use light_compressed_account::compressed_account::CompressedAccountWithMerkleContext;
pub use light_ctoken_types::state::TokenData;
use light_ctoken_types::state::TokenDataVersion;
use light_sdk::{
    instruction::PackedAccounts,
    light_hasher::{sha256::Sha256BE, HasherError},
};
use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;

use crate::{AnchorDeserialize, AnchorSerialize};

// We define the traits here to circumvent the orphan rule.
pub trait Pack {
    type Packed;
    fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Self::Packed;
}
pub trait Unpack {
    type Unpacked;
    fn unpack(
        &self,
        remaining_accounts: &[AccountInfo],
    ) -> std::result::Result<Self::Unpacked, ProgramError>;
}

impl Pack for TokenData {
    type Packed = light_ctoken_types::instructions::transfer2::MultiTokenTransferOutputData;

    fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Self::Packed {
        Self::Packed {
            owner: remaining_accounts.insert_or_get(self.owner.to_bytes().into()),
            mint: remaining_accounts.insert_or_get_read_only(self.mint.to_bytes().into()),
            amount: self.amount,
            has_delegate: self.delegate.is_some(),
            delegate: if let Some(delegate) = self.delegate {
                remaining_accounts.insert_or_get(delegate.to_bytes().into())
            } else {
                0
            },
            version: TokenDataVersion::ShaFlat as u8,
        }
    }
}

impl Unpack for TokenData {
    type Unpacked = Self;

    fn unpack(
        &self,
        _remaining_accounts: &[AccountInfo],
    ) -> std::result::Result<Self::Unpacked, ProgramError> {
        Ok(self.clone())
    }
}

/// Solana-compatible token types using `solana_pubkey::Pubkey`
pub mod compat {
    use solana_pubkey::Pubkey;

    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorDeserialize, AnchorSerialize, Default)]
    #[repr(u8)]
    pub enum AccountState {
        #[default]
        Initialized = 0,
        Frozen = 1,
    }

    impl From<AccountState> for light_ctoken_types::state::CompressedTokenAccountState {
        fn from(state: AccountState) -> Self {
            match state {
                AccountState::Initialized => {
                    light_ctoken_types::state::CompressedTokenAccountState::Initialized
                }
                AccountState::Frozen => {
                    light_ctoken_types::state::CompressedTokenAccountState::Frozen
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
    /// For zero-copy operations, use [`TokenData`](crate::types::TokenData) from the crate root.
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
        /// Placeholder for TokenExtension tlv data (unimplemented)
        pub tlv: Option<Vec<u8>>,
    }

    impl TokenData {
        /// TokenDataVersion 3
        /// CompressedAccount Discriminator [0,0,0,0,0,0,0,4]
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

    impl From<TokenData> for crate::pack::TokenData {
        fn from(data: TokenData) -> Self {
            use light_ctoken_types::state::CompressedTokenAccountState;

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

    impl From<crate::pack::TokenData> for TokenData {
        fn from(data: crate::pack::TokenData) -> Self {
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

    impl Pack for TokenData {
        type Packed = InputTokenDataCompressible;

        fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Self::Packed {
            InputTokenDataCompressible {
                owner: remaining_accounts.insert_or_get(self.owner),
                mint: remaining_accounts.insert_or_get_read_only(self.mint),
                amount: self.amount,
                has_delegate: self.delegate.is_some(),
                delegate: if let Some(delegate) = self.delegate {
                    remaining_accounts.insert_or_get(delegate)
                } else {
                    0
                },
                version: TokenDataVersion::ShaFlat as u8,
            }
        }
    }

    impl Unpack for TokenData {
        type Unpacked = Self;

        fn unpack(
            &self,
            _remaining_accounts: &[AccountInfo],
        ) -> std::result::Result<Self::Unpacked, ProgramError> {
            Ok(self.clone())
        }
    }

    impl Unpack for InputTokenDataCompressible {
        type Unpacked = TokenData;

        fn unpack(
            &self,
            remaining_accounts: &[AccountInfo],
        ) -> std::result::Result<Self::Unpacked, ProgramError> {
            Ok(TokenData {
                owner: *remaining_accounts
                    .get(self.owner as usize)
                    .ok_or(ProgramError::InvalidAccountData)?
                    .key,
                amount: self.amount,
                delegate: if self.has_delegate {
                    Some(
                        *remaining_accounts
                            .get(self.delegate as usize)
                            .ok_or(ProgramError::InvalidAccountData)?
                            .key,
                    )
                } else {
                    None
                },
                mint: *remaining_accounts
                    .get(self.mint as usize)
                    .ok_or(ProgramError::InvalidAccountData)?
                    .key,
                state: AccountState::Initialized,
                tlv: None,
            })
        }
    }

    /// Wrapper for token data with variant information
    #[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
    pub struct TokenDataWithVariant<V> {
        pub variant: V,
        pub token_data: TokenData,
    }

    #[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
    pub struct PackedCTokenDataWithVariant<V> {
        pub variant: V,
        pub token_data: InputTokenDataCompressible,
    }

    #[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
    pub struct CTokenDataWithVariant<V> {
        pub variant: V,
        pub token_data: TokenData,
    }

    impl<V> Pack for CTokenDataWithVariant<V>
    where
        V: AnchorSerialize + Clone + std::fmt::Debug,
    {
        type Packed = PackedCTokenDataWithVariant<V>;

        fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Self::Packed {
            PackedCTokenDataWithVariant {
                variant: self.variant.clone(),
                token_data: self.token_data.pack(remaining_accounts),
            }
        }
    }

    impl<V> Unpack for CTokenDataWithVariant<V>
    where
        V: Clone,
    {
        type Unpacked = TokenDataWithVariant<V>;

        fn unpack(
            &self,
            remaining_accounts: &[AccountInfo],
        ) -> std::result::Result<Self::Unpacked, ProgramError> {
            Ok(TokenDataWithVariant {
                variant: self.variant.clone(),
                token_data: self.token_data.unpack(remaining_accounts)?,
            })
        }
    }

    impl<V> Pack for TokenDataWithVariant<V>
    where
        V: AnchorSerialize + Clone + std::fmt::Debug,
    {
        type Packed = PackedCTokenDataWithVariant<V>;

        fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Self::Packed {
            PackedCTokenDataWithVariant {
                variant: self.variant.clone(),
                token_data: self.token_data.pack(remaining_accounts),
            }
        }
    }

    impl<V> Unpack for PackedCTokenDataWithVariant<V>
    where
        V: Clone,
    {
        type Unpacked = TokenDataWithVariant<V>;

        fn unpack(
            &self,
            remaining_accounts: &[AccountInfo],
        ) -> std::result::Result<Self::Unpacked, ProgramError> {
            Ok(TokenDataWithVariant {
                variant: self.variant.clone(),
                token_data: self.token_data.unpack(remaining_accounts)?,
            })
        }
    }

    // TODO: remove aliases in separate PR
    pub type InputTokenDataCompressible =
        light_ctoken_types::instructions::transfer2::MultiTokenTransferOutputData;
    pub type CompressibleTokenDataWithVariant<V> = CTokenDataWithVariant<V>;
    pub type PackedCompressibleTokenDataWithVariant<V> = PackedCTokenDataWithVariant<V>;
    pub type CTokenData<V> = CTokenDataWithVariant<V>;
    pub type PackedCTokenData<V> = PackedCTokenDataWithVariant<V>;
}
