use light_compressed_account::compressed_account::CompressedAccountWithMerkleContext;
use light_hasher::{sha256::Sha256BE, HasherError};

use crate::{AnchorDeserialize, AnchorSerialize, Pubkey};

#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorDeserialize, AnchorSerialize, Default)]
#[repr(u8)]
pub enum AccountState {
    #[default]
    Initialized,
    Frozen,
}
// TODO: extract token data from program into into a separate crate, import it and remove this file.
#[derive(Debug, PartialEq, Eq, AnchorDeserialize, AnchorSerialize, Clone, Default)]
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
        use light_hasher::Hasher;
        let bytes = self.try_to_vec().map_err(|_| HasherError::BorshError)?;
        Sha256BE::hash(bytes.as_slice())
    }
}
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

/// Implementation for TokenData - packs into InputTokenDataCompressible
impl Pack for TokenData {
    type Packed = InputTokenDataCompressible;

    fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Self::Packed {
        InputTokenDataCompressible {
            owner: remaining_accounts.insert_or_get(self.owner),
            amount: self.amount,
            has_delegate: self.delegate.is_some(),
            delegate: if let Some(delegate) = self.delegate {
                remaining_accounts.insert_or_get(delegate)
            } else {
                0 // Unused when has_delegate is false
            },
            mint: remaining_accounts.insert_or_get_read_only(self.mint),
            version: 3, // TokenDataVersion::ShaFlat. Default version for compressed token accounts
        }
    }
}

impl Unpack for TokenData {
    type Unpacked = Self;

    fn unpack(
        &self,
        _remaining_accounts: &[AccountInfo],
    ) -> std::result::Result<Self::Unpacked, solana_program_error::ProgramError> {
        Ok(self.clone())
    }
}

/// Unpack implementation for InputTokenDataCompressible
impl Unpack for InputTokenDataCompressible {
    type Unpacked = TokenData;

    fn unpack(
        &self,
        remaining_accounts: &[AccountInfo],
    ) -> std::result::Result<Self::Unpacked, solana_program_error::ProgramError> {
        Ok(TokenData {
            owner: *remaining_accounts
                .get(self.owner as usize)
                .ok_or(solana_program_error::ProgramError::InvalidAccountData)?
                .key,
            amount: self.amount,
            delegate: if self.has_delegate {
                Some(
                    *remaining_accounts
                        .get(self.delegate as usize)
                        .ok_or(solana_program_error::ProgramError::InvalidAccountData)?
                        .key,
                )
            } else {
                None
            },
            mint: *remaining_accounts
                .get(self.mint as usize)
                .ok_or(solana_program_error::ProgramError::InvalidAccountData)?
                .key,
            state: AccountState::Initialized, // Default state for unpacked
            tlv: None,                        // No TLV data in packed version
        })
    }
}

/// Wrapper for token data with variant information
/// The variant is user-defined and doesn't get altered during packing
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

/// Pack implementation for CTokenDataWithVariant
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

/// Unpack implementation for CTokenDataWithVariant
impl<V> Unpack for CTokenDataWithVariant<V>
where
    V: Clone,
{
    type Unpacked = TokenDataWithVariant<V>;

    fn unpack(
        &self,
        remaining_accounts: &[AccountInfo],
    ) -> std::result::Result<Self::Unpacked, solana_program_error::ProgramError> {
        Ok(TokenDataWithVariant {
            variant: self.variant.clone(),
            token_data: self.token_data.unpack(remaining_accounts)?,
        })
    }
}

/// Pack implementation for TokenDataWithVariant
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

/// Unpack implementation for PackedCTokenDataWithVariant
impl<V> Unpack for PackedCTokenDataWithVariant<V>
where
    V: Clone,
{
    type Unpacked = TokenDataWithVariant<V>;

    fn unpack(
        &self,
        remaining_accounts: &[AccountInfo],
    ) -> std::result::Result<Self::Unpacked, solana_program_error::ProgramError> {
        Ok(TokenDataWithVariant {
            variant: self.variant.clone(),
            token_data: self.token_data.unpack(remaining_accounts)?,
        })
    }
}

// custom replacement for MultiInputTokenDataWithContext
// without root_index and without merkle_context
#[derive(Clone, Debug, AnchorDeserialize, AnchorSerialize, Default)]
pub struct InputTokenDataCompressible {
    pub owner: u8,
    pub amount: u64,
    pub has_delegate: bool, // Optional delegate is set
    pub delegate: u8,
    pub mint: u8,
    pub version: u8,
}

// TODO: remove these and fix renaming after we're done with ci.
#[deprecated(since = "0.2.0", note = "Use `CTokenDataWithVariant` instead")]
pub type CompressibleTokenDataWithVariant<V> = CTokenDataWithVariant<V>;

#[deprecated(since = "0.2.0", note = "Use `PackedCTokenDataWithVariant` instead")]
pub type PackedCompressibleTokenDataWithVariant<V> = PackedCTokenDataWithVariant<V>;

// Shorter aliases for convenience
pub type CTokenData<V> = CTokenDataWithVariant<V>;
pub type PackedCTokenData<V> = PackedCTokenDataWithVariant<V>;
