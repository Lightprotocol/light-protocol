//! Pack implementation for TokenData types for c-tokens.
use light_compressed_account::compressed_account::CompressedAccountWithMerkleContext;
use light_sdk::{instruction::PackedAccounts, light_hasher::HasherError};
pub use light_token_interface::state::TokenData;
use light_token_interface::state::TokenDataVersion;
use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;

use crate::{AnchorDeserialize, AnchorSerialize};

// Note: We define Pack/Unpack traits locally to circumvent the orphan rule.
// This allows implementing them for external types like TokenData from ctoken-interface.
// The sdk has identical trait definitions in light_sdk::interface.
pub trait Pack {
    type Packed;
    fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Result<Self::Packed, ProgramError>;
}
pub trait Unpack {
    type Unpacked;
    fn unpack(
        &self,
        remaining_accounts: &[AccountInfo],
    ) -> std::result::Result<Self::Unpacked, ProgramError>;
}

/// Solana-compatible token types using `solana_pubkey::Pubkey`
pub mod compat {
    // Re-export TokenData and AccountState from compressed-token-sdk for type compatibility
    pub use light_compressed_token_sdk::compat::{
        AccountState, InputTokenDataCompressible, TokenData,
    };

    use super::*;

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

    impl Pack for TokenData {
        type Packed = InputTokenDataCompressible;

        fn pack(
            &self,
            remaining_accounts: &mut PackedAccounts,
        ) -> Result<Self::Packed, ProgramError> {
            Ok(InputTokenDataCompressible {
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
            })
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
    pub struct PackedTokenDataWithVariant<V> {
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
        V: Pack,
        V::Packed: AnchorSerialize + Clone + std::fmt::Debug,
    {
        type Packed = PackedTokenDataWithVariant<V::Packed>;

        fn pack(
            &self,
            remaining_accounts: &mut PackedAccounts,
        ) -> Result<Self::Packed, ProgramError> {
            Ok(PackedTokenDataWithVariant {
                variant: self.variant.pack(remaining_accounts)?,
                token_data: self.token_data.pack(remaining_accounts)?,
            })
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
            // Note: This impl assumes V is already unpacked (has Pubkeys).
            // For packed variants, use PackedTokenDataWithVariant::unpack instead.
            Ok(TokenDataWithVariant {
                variant: self.variant.clone(),
                token_data: self.token_data.unpack(remaining_accounts)?,
            })
        }
    }

    impl<V> Pack for TokenDataWithVariant<V>
    where
        V: Pack,
        V::Packed: AnchorSerialize + Clone + std::fmt::Debug,
    {
        type Packed = PackedTokenDataWithVariant<V::Packed>;

        fn pack(
            &self,
            remaining_accounts: &mut PackedAccounts,
        ) -> Result<Self::Packed, ProgramError> {
            Ok(PackedTokenDataWithVariant {
                variant: self.variant.pack(remaining_accounts)?,
                token_data: self.token_data.pack(remaining_accounts)?,
            })
        }
    }

    impl<V> Unpack for PackedTokenDataWithVariant<V>
    where
        V: Unpack,
    {
        type Unpacked = TokenDataWithVariant<V::Unpacked>;

        fn unpack(
            &self,
            remaining_accounts: &[AccountInfo],
        ) -> std::result::Result<Self::Unpacked, ProgramError> {
            Ok(TokenDataWithVariant {
                variant: self.variant.unpack(remaining_accounts)?,
                token_data: self.token_data.unpack(remaining_accounts)?,
            })
        }
    }

    // TODO: remove aliases in separate PR
    pub type CompressibleTokenDataWithVariant<V> = CTokenDataWithVariant<V>;
    pub type PackedCompressibleTokenDataWithVariant<V> = PackedTokenDataWithVariant<V>;
    pub type CTokenData<V> = CTokenDataWithVariant<V>;
    pub type PackedCTokenData<V> = PackedTokenDataWithVariant<V>;
}
