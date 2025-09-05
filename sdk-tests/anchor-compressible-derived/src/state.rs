use anchor_lang::prelude::*;
use light_sdk::{compressible::CompressionInfo, LightDiscriminator, LightHasher};
use light_sdk::{Compressible, CompressiblePack};

#[derive(
    Debug, LightHasher, LightDiscriminator, Compressible, CompressiblePack, Default, InitSpace,
)]
#[light_seeds(b"user_record", owner.as_ref())]
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
    Debug, LightHasher, LightDiscriminator, Default, InitSpace, Compressible, CompressiblePack,
)]
#[light_seeds(b"game_session", session_id.to_le_bytes().as_ref())]
#[compress_as(
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

// PlaceholderRecord - demonstrates empty compressed account creation
#[derive(
    Debug, LightHasher, LightDiscriminator, Default, InitSpace, Compressible, CompressiblePack,
)]
#[light_seeds(b"placeholder_record", placeholder_id.to_le_bytes().as_ref())]
#[account]
pub struct PlaceholderRecord {
    #[skip]
    pub compression_info: Option<CompressionInfo>,
    #[hash]
    pub owner: Pubkey,
    #[hash]
    #[max_len(32)]
    pub name: String,
    pub placeholder_id: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy)]
#[repr(u8)]
pub enum CTokenAccountVariant {
    CTokenSigner = 0,
    AssociatedTokenAccount = 255, // TODO: add support.
}
