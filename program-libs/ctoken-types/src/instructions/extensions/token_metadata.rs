use light_compressed_account::Pubkey;
use light_zero_copy::ZeroCopy;

use crate::{
    state::{AdditionalMetadata, Metadata},
    AnchorDeserialize, AnchorSerialize,
};

// TODO: double check hashing scheme, add tests with partial data
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct TokenMetadataInstructionData {
    pub update_authority: Option<Pubkey>,
    pub metadata: Metadata,
    pub additional_metadata: Option<Vec<AdditionalMetadata>>,
    pub version: u8,
}
