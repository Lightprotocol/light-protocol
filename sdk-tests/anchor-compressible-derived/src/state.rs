use anchor_lang::prelude::*;
use light_sdk::{compressible::CompressionInfo, LightDiscriminator, LightHasher};
use light_sdk_macros::HasCompressionInfo;

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

#[derive(Debug, LightHasher, LightDiscriminator, Default, InitSpace, HasCompressionInfo)]
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
