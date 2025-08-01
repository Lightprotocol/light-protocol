use light_hasher::{Hasher, Poseidon, Sha256};
pub mod compressible;
//pub mod metadata_pointer;
pub mod token_metadata;
use pinocchio::log::sol_log_compute_units;
use solana_msg::msg;
//pub use metadata_pointer::{InitMetadataPointer, ZInitMetadataPointer};
pub use token_metadata::{TokenMetadataInstructionData, ZTokenMetadataInstructionData};

use crate::{
    hash_cache::HashCache, state::Version, AnchorDeserialize, AnchorSerialize, CTokenError,
};

#[derive(Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
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
    Placeholder18, // MetadataPointer(InitMetadataPointer),
    TokenMetadata(TokenMetadataInstructionData),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ZExtensionInstructionData<'a> {
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
    Placeholder18, // MetadataPointer(ZInitMetadataPointer<'a>),
    TokenMetadata(ZTokenMetadataInstructionData<'a>),
}

impl ExtensionInstructionData {
    pub fn hash<H: Hasher>(
        &self,
        mint: light_compressed_account::Pubkey,
        context: &mut HashCache,
    ) -> Result<[u8; 32], CTokenError> {
        match self {
            /* ExtensionInstructionData::MetadataPointer(metadata_pointer) => {
                metadata_pointer.hash_metadata_pointer::<H>(context)
            }*/
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
            /*ZExtensionInstructionData::MetadataPointer(metadata_pointer) => {
                metadata_pointer.hash_metadata_pointer::<H>(context)
            }*/
            ZExtensionInstructionData::TokenMetadata(token_metadata) => {
                match Version::try_from(token_metadata.version)? {
                    Version::Poseidon => {
                        // TODO: cleanup other hashing code
                        msg!("poseidon");
                        sol_log_compute_units();
                        let hash =
                            token_metadata.hash_token_metadata::<Poseidon>(hashed_mint, context);
                        sol_log_compute_units();
                        hash
                    }
                    Version::Sha256 => {
                        msg!("sha256");
                        sol_log_compute_units();
                        let mut hash =
                            token_metadata.hash_token_metadata::<Sha256>(hashed_mint, context)?;
                        sol_log_compute_units();
                        hash[0] = 0;
                        Ok(hash)
                    }
                    _ => {
                        msg!(
                            "TokenMetadata hash version not supported {} (0 Poseidon, 1 Sha256 are supported).",
                            token_metadata.version
                        );
                        unimplemented!(
                            "TokenMetadata hash version not supported {}",
                            token_metadata.version
                        )
                    } // Version::Keccak256 => <Self as DataHasher>::hash::<Keccak>(self),
                      // Version::Sha256Flat => self.sha_flat(),
                }
            }
            _ => Err(CTokenError::UnsupportedExtension),
        }
    }
}

// Manual implementation of zero-copy traits for ExtensionInstructionData
impl<'a> light_zero_copy::borsh::Deserialize<'a> for ExtensionInstructionData {
    type Output = ZExtensionInstructionData<'a>;

    fn zero_copy_at(
        data: &'a [u8],
    ) -> Result<(Self::Output, &'a [u8]), light_zero_copy::errors::ZeroCopyError> {
        // Read discriminant (first 1 byte for borsh enum)
        if data.is_empty() {
            return Err(light_zero_copy::errors::ZeroCopyError::ArraySize(
                1,
                data.len(),
            ));
        }

        let discriminant = data[0];
        let remaining_data = &data[1..];

        match discriminant {
            /* 18 => {
                let (metadata_pointer, remaining_bytes) =
                    InitMetadataPointer::zero_copy_at(remaining_data)?;
                Ok((
                    ZExtensionInstructionData::MetadataPointer(metadata_pointer),
                    remaining_bytes,
                ))
            }*/
            19 => {
                let (token_metadata, remaining_bytes) =
                    TokenMetadataInstructionData::zero_copy_at(remaining_data)?;
                Ok((
                    ZExtensionInstructionData::TokenMetadata(token_metadata),
                    remaining_bytes,
                ))
            }
            _ => Err(light_zero_copy::errors::ZeroCopyError::InvalidConversion),
        }
    }
}
