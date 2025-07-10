use anchor_lang::prelude::*;
use light_sdk::{
    compressible::{CompressionInfo, HasCompressionInfo},
    derive_light_cpi_signer, LightDiscriminator, LightHasher,
};
use light_sdk_macros::add_compressible_instructions;
use light_sdk_types::CpiSigner;

declare_id!("CompUser11111111111111111111111111111111111");
// pub const ADDRESS_SPACE: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");
// pub const RENT_RECIPIENT: Pubkey = pubkey!("CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG");
pub const COMPRESSION_DELAY: u32 = 100;
pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("GRLu2hKaAiMbxpkAM1HeXzks9YeGuz18SEgXEizVvPqX");

#[add_compressible_instructions(UserRecord, GameSession)]
#[program]
pub mod anchor_compressible_user_derived {
    use super::*;

    // The macro will generate:
    // - create_compression_config (config management)
    // - update_compression_config (config management)
    // - compress_user_record (compress existing PDA)
    // - compress_game_session (compress existing PDA)
    // - decompress_multiple_pdas (decompress compressed accounts)
    // Plus all the necessary structs and enums
    //
    // NOTE: create_user_record and create_game_session are NOT generated
    // because they typically need custom initialization logic
}

#[derive(Debug, LightHasher, LightDiscriminator, Default, InitSpace)]
#[account]
pub struct UserRecord {
    #[skip]
    pub compression_info: CompressionInfo,
    #[hash]
    pub owner: Pubkey,
    #[hash]
    #[max_len(32)]
    pub name: String,
    pub score: u64,
}

impl HasCompressionInfo for UserRecord {
    fn compression_info(&self) -> &CompressionInfo {
        &self.compression_info
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        &mut self.compression_info
    }
}

#[derive(Debug, LightHasher, LightDiscriminator, Default, InitSpace)]
#[account]
pub struct GameSession {
    #[skip]
    pub compression_info: CompressionInfo,
    pub session_id: u64,
    #[hash]
    pub player: Pubkey,
    #[hash]
    #[max_len(32)]
    pub game_type: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub score: u64,
}

impl HasCompressionInfo for GameSession {
    fn compression_info(&self) -> &CompressionInfo {
        &self.compression_info
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        &mut self.compression_info
    }
}
