use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::{
    compressible::{CompressionInfo, HasCompressionInfo},
    LightDiscriminator, LightHasher,
};
use solana_program::account_info::AccountInfo;
use solana_program::pubkey::Pubkey;

pub const COMPRESSION_DELAY: u64 = 100;

// Decompress a PDA into an account
pub fn decompress_dynamic_pda(
    _accounts: &[AccountInfo],
    _inputs: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Implementation would go here
    Ok(())
}

#[derive(
    Default, Clone, Debug, BorshSerialize, BorshDeserialize, LightHasher, LightDiscriminator,
)]
pub struct MyPdaAccount {
    #[skip]
    pub compression_info: CompressionInfo,
    #[hash]
    pub owner: Pubkey,
    pub data: u64,
}

// Implement the HasCompressionInfo trait
impl HasCompressionInfo for MyPdaAccount {
    fn compression_info(&self) -> &CompressionInfo {
        &self.compression_info
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        &mut self.compression_info
    }
}
