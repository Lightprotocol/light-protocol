use light_compressed_account::Pubkey;
use light_zero_copy::ZeroCopy;

use crate::{state::AdditionalMetadata, AnchorDeserialize, AnchorSerialize};

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize, ZeroCopy)]
pub struct TokenMetadataInstructionData {
    pub update_authority: Option<Pubkey>,
    pub name: Vec<u8>,
    pub symbol: Vec<u8>,
    pub uri: Vec<u8>,
    pub additional_metadata: Option<Vec<AdditionalMetadata>>,
}
