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
