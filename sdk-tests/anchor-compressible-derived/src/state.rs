use anchor_lang::prelude::*;
use light_sdk::{compressible::CompressionInfo, LightDiscriminator, LightHasher};
use light_sdk_macros::{CompressAs, HasCompressionInfo};

#[derive(Debug, LightHasher, LightDiscriminator, HasCompressionInfo, Default, InitSpace)]
#[account]
pub struct UserRecord {
    #[skip]
    pub compression_info: Option<CompressionInfo>,
    #[hash]
    pub owner: Pubkey,
    #[hash]
    #[max_len(32)]
    pub name: String,
    pub score: u64,
}

#[derive(
    Debug, LightHasher, LightDiscriminator, Default, InitSpace, HasCompressionInfo, CompressAs,
)]
#[compressible_as(
    start_time = 0,
    end_time = None,
    score = 0
    // session_id, player, game_type, compression_info are kept as-is
)]
#[account]
pub struct GameSession {
    #[skip]
    pub compression_info: Option<CompressionInfo>,
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
