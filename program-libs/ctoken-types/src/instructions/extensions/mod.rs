use light_hasher::{sha256::Sha256BE, Hasher, Poseidon};
pub mod compressible;
pub mod token_metadata;
use light_zero_copy::ZeroCopy;
use solana_msg::msg;
pub use token_metadata::{TokenMetadataInstructionData, ZTokenMetadataInstructionData};

use crate::{
    hash_cache::HashCache, state::Version, AnchorDeserialize, AnchorSerialize, CTokenError,
    HashableExtension,
};

#[derive(Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
#[repr(C)]
pub enum ExtensionInstructionData {
    Placeholder0,
    Placeholder1,
    Placeholder2,
    Placeholder3,
    Placeholder4,
    Placeholder5,
    Placeholder6,
    Placeholder7,
    Placeholder8,
    Placeholder9,
    Placeholder10,
    Placeholder11,
    Placeholder12,
    Placeholder13,
    Placeholder14,
    Placeholder15,
    Placeholder16,
    Placeholder17,
    Placeholder18,
    TokenMetadata(TokenMetadataInstructionData),
}

impl ExtensionInstructionData {
    pub fn hash<H: Hasher>(
        &self,
        mint: light_compressed_account::Pubkey,
        context: &mut HashCache,
    ) -> Result<[u8; 32], CTokenError> {
        match self {
            ExtensionInstructionData::TokenMetadata(token_metadata) => {
                token_metadata.hash_token_metadata::<H>(mint, context)
            }
            _ => Err(CTokenError::UnsupportedExtension),
        }
    }
}

impl ZExtensionInstructionData<'_> {
    pub fn hash<H: Hasher>(
        &self,
        hashed_mint: &[u8; 32],
        context: &mut HashCache,
    ) -> Result<[u8; 32], CTokenError> {
        match self {
            ZExtensionInstructionData::TokenMetadata(token_metadata) => {
                match Version::try_from(token_metadata.version)? {
                    Version::Poseidon => {
                        token_metadata.hash_token_metadata::<Poseidon>(hashed_mint, context)
                    }
                    Version::Sha256 => {
                        Ok(token_metadata.hash_token_metadata::<Sha256BE>(hashed_mint, context)?)
                    }
                    _ => {
                        msg!(
                            "TokenMetadata hash version not supported {} (0 Poseidon, 1 Sha256 are supported).",
                            token_metadata.version
                        );
                        Err(CTokenError::UnsupportedExtension)
                    } // Version::Keccak256 => <Self as DataHasher>::hash::<Keccak>(self),
                      // Version::Sha256Flat => self.sha_flat(),
                }
            }
            _ => Err(CTokenError::UnsupportedExtension),
        }
    }
}

impl HashableExtension<CTokenError> for ZExtensionInstructionData<'_> {
    fn hash_with_hasher<H: Hasher>(
        &self,
        hashed_spl_mint: &[u8; 32],
        hash_cache: &mut HashCache,
    ) -> Result<[u8; 32], CTokenError> {
        match self {
            ZExtensionInstructionData::TokenMetadata(token_metadata) => {
                token_metadata.hash_token_metadata::<H>(hashed_spl_mint, hash_cache)
            }
            _ => Err(CTokenError::UnsupportedExtension),
        }
    }
}
