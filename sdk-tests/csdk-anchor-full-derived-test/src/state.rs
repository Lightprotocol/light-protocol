use anchor_lang::prelude::*;
use light_ctoken_interface::instructions::mint_action::CompressedMintWithContext;
use light_sdk::{
    compressible::CompressionInfo,
    instruction::{PackedAddressTreeInfo, ValidityProof},
    LightDiscriminator,
};
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

/// Test struct using the new consolidated LightCompressible derive.
/// This demonstrates that `#[derive(LightCompressible)]` is equivalent to
/// `#[derive(LightHasherSha, LightDiscriminator, Compressible, CompressiblePack)]`
/// No #[hash] or #[skip] attributes needed - SHA256 hashes entire struct, compression_info auto-skipped.
#[derive(Default, Debug, InitSpace, LightCompressible)]
#[account]
pub struct NewStyleRecord {
    pub compression_info: Option<CompressionInfo>,
    pub owner: Pubkey,
    #[max_len(128)]
    pub metadata: String,
    pub version: u32,
    pub flags: u64,
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

/// E2E test params: creates 2 PDAs + light mint + decompress + CToken vault + user ATA + mint
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct E2eTestParams {
    pub proof: ValidityProof,
    // PlaceholderRecord params
    pub placeholder_compressed_address: [u8; 32],
    pub placeholder_address_tree_info: PackedAddressTreeInfo,
    pub placeholder_output_state_tree_index: u8,
    // UserRecord params (2nd PDA)
    pub user_record_compressed_address: [u8; 32],
    pub user_record_address_tree_info: PackedAddressTreeInfo,
    pub user_record_output_state_tree_index: u8,
    // Light mint params
    pub mint_with_context: CompressedMintWithContext,
    pub mint_address_tree_info: PackedAddressTreeInfo,
    pub cmint_bump: u8,
    // Decompress params
    pub rent_payment: u8,
    pub write_top_up: u32,
    // CToken params
    pub user_ata_bump: u8, // User ATA bump for CPI creation
    // Mint amounts
    pub vault_mint_amount: u64,    // Amount to mint to vault
    pub user_ata_mint_amount: u64, // Amount to mint to user's ATA
}

/// Simple data for E2E test
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct E2eTestData {
    pub placeholder_name: String,
    pub placeholder_id: u64,
    pub counter: u32,
    pub mint_decimals: u8,
    // UserRecord data
    pub user_record_owner: Pubkey,
    pub user_record_name: String,
    pub user_record_score: u64,
    pub user_record_category_id: u64,
}
