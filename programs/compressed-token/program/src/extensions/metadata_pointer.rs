use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::Pubkey;
use light_sdk::LightHasher;
use light_zero_copy::ZeroCopy;

/// Metadata pointer extension data for compressed mints.
#[derive(Debug, Clone, PartialEq, BorshSerialize, ZeroCopy, BorshDeserialize, LightHasher)]
pub struct MetadataPointer {
    /// Authority that can set the metadata address
    #[hash]
    pub authority: Option<Pubkey>,
    /// Compressed address that holds the metadata (in token 22)
    #[hash]
    // TODO: implement manually, because there is no need to hash the compressed metadata_address
    pub metadata_address: Option<Pubkey>,
}

#[derive(
    Debug, PartialEq, Default, Clone, Copy, Eq, BorshSerialize, BorshDeserialize, ZeroCopy,
)]
pub struct NewAddressParamsAssignedPackedWithAddress {
    pub address: [u8; 32],
    pub seed: [u8; 32],
    pub address_merkle_tree_account_index: u8,
    pub address_merkle_tree_root_index: u16,
}

impl MetadataPointer {
    /// Validate metadata pointer - at least one field must be provided
    pub fn validate(&self) -> Result<(), anchor_lang::prelude::ProgramError> {
        if self.authority.is_none() && self.metadata_address.is_none() {
            return Err(anchor_lang::prelude::ProgramError::InvalidInstructionData);
        }
        Ok(())
    }
}

/// Instruction data for initializing metadata pointer
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, ZeroCopy)]
pub struct InitializeMetadataPointerInstructionData {
    /// The authority that can set the metadata address
    pub authority: Option<Pubkey>,
    /// The account address that holds the metadata
    pub metadata_address_params: Option<NewAddressParamsAssignedPackedWithAddress>,
}
