use anchor_lang::prelude::*;
use light_sdk::{
    compressible::CompressionInfo,
    instruction::{PackedAddressTreeInfo, ValidityProof},
    LightDiscriminator,
};
use light_sdk_macros::RentFreeAccount;
use light_token_interface::instructions::mint_action::MintWithContext;

#[derive(Default, Debug, InitSpace, RentFreeAccount)]
#[account]
pub struct UserRecord {
    pub compression_info: Option<CompressionInfo>,
    pub owner: Pubkey,
    #[max_len(32)]
    pub name: String,
    pub score: u64,
    pub category_id: u64,
}

#[derive(Default, Debug, InitSpace, RentFreeAccount)]
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

#[derive(Default, Debug, InitSpace, RentFreeAccount)]
#[account]
pub struct PlaceholderRecord {
    pub compression_info: Option<CompressionInfo>,
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
    pub mint_with_context: MintWithContext,
}
