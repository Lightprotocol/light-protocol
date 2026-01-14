use anchor_lang::prelude::*;
use light_sdk::{compressible::CompressionInfo, LightDiscriminator};
use light_sdk_macros::LightCompressible;

// Using LightHasherSha for SHA256 - no #[hash] or #[skip] needed
#[derive(Default, Debug, InitSpace, LightCompressible)]
#[account]
pub struct UserRecord {
    pub compression_info: Option<CompressionInfo>,
    pub owner: Pubkey,
    #[max_len(32)]
    pub name: String,
    pub score: u64,
    pub category_id: u64,
}

#[derive(Default, Debug, InitSpace, LightCompressible)]
#[compress_as(start_time = 0, end_time = None, score = 0)]
#[account]
pub struct GameSession {
    pub compression_info: Option<CompressionInfo>,
    pub session_id: u64,
    pub player: Pubkey,
    #[max_len(32)]
    pub game_type: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub score: u64,
}

#[derive(Default, Debug, InitSpace, LightCompressible)]
#[account]
pub struct PlaceholderRecord {
    pub compression_info: Option<CompressionInfo>,
    pub owner: Pubkey,
    #[max_len(32)]
    pub name: String,
    pub placeholder_id: u64,
    pub counter: u32,
}
