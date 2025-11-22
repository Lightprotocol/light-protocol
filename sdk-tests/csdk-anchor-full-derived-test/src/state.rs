use anchor_lang::prelude::*;
use light_ctoken_types::instructions::mint_action::CompressedMintWithContext;
use light_sdk::{
    compressible::CompressionInfo,
    instruction::{PackedAddressTreeInfo, ValidityProof},
    LightDiscriminator, LightHasher,
};
use light_sdk_macros::{Compressible, CompressiblePack};

#[derive(
    Default, Debug, LightHasher, LightDiscriminator, InitSpace, Compressible, CompressiblePack,
)]
#[account]
pub struct UserRecord {
    #[skip]
    pub compression_info: Option<CompressionInfo>,
    #[hash]
    pub owner: Pubkey,
    #[max_len(32)]
    pub name: String,
    pub score: u64,
    pub category_id: u64,
}

#[derive(
    Default, Debug, LightHasher, LightDiscriminator, InitSpace, Compressible, CompressiblePack,
)]
#[compress_as(start_time = 0, end_time = None, score = 0)]
#[account]
pub struct GameSession {
    #[skip]
    pub compression_info: Option<CompressionInfo>,
    pub session_id: u64,
    #[hash]
    pub player: Pubkey,
    #[max_len(32)]
    pub game_type: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub score: u64,
}

#[derive(
    Default, Debug, LightHasher, LightDiscriminator, InitSpace, Compressible, CompressiblePack,
)]
#[account]
pub struct PlaceholderRecord {
    #[skip]
    pub compression_info: Option<CompressionInfo>,
    #[hash]
    pub owner: Pubkey,
    #[max_len(32)]
    pub name: String,
    pub placeholder_id: u64,
    pub counter: u32,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct AccountCreationData {
    // Instruction data fields (accounts come from ctx.accounts.*)
    pub owner: Pubkey,
    pub category_id: u64,
    pub user_name: String,
    pub session_id: u64,
    pub game_type: String,
    pub placeholder_id: u64,
    pub counter: u32,
    pub mint_name: String,
    pub mint_symbol: String,
    pub mint_uri: String,
    pub mint_decimals: u8,
    pub mint_supply: u64,
    pub mint_update_authority: Option<Pubkey>,
    pub mint_freeze_authority: Option<Pubkey>,
    pub additional_metadata: Option<Vec<(String, String)>>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct TokenAccountInfo {
    pub user: Pubkey,
    pub mint: Pubkey,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CompressionParams {
    pub proof: ValidityProof,
    pub user_compressed_address: [u8; 32],
    pub user_address_tree_info: PackedAddressTreeInfo,
    pub user_output_state_tree_index: u8,
    pub game_compressed_address: [u8; 32],
    pub game_address_tree_info: PackedAddressTreeInfo,
    pub game_output_state_tree_index: u8,
    pub mint_bump: u8,
    pub mint_with_context: CompressedMintWithContext,
}
