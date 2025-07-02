use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::Pubkey;
use light_ctoken_types::instructions::extensions::compressible::CompressibleExtensionInstructionData;
use light_zero_copy::ZeroCopy;

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, ZeroCopy)]
pub struct CreateTokenAccountInstructionData {
    /// The owner of the token account
    pub owner: Pubkey,
    /// Optional compressible configuration for the token account
    pub compressible_config: Option<CompressibleExtensionInstructionData>,
}
