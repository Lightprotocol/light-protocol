//! AMM state structs adapted from cp-swap-reference.

use anchor_lang::prelude::*;
use light_account::{CompressionInfo, LightDiscriminator};
use light_sdk_macros::LightAccount;

pub const POOL_SEED: &str = "pool";
pub const POOL_VAULT_SEED: &str = "pool_vault";
pub const OBSERVATION_SEED: &str = "observation";
pub const POOL_LP_MINT_SIGNER_SEED: &[u8] = b"pool_lp_mint";
pub const AUTH_SEED: &str = "vault_and_lp_mint_auth_seed";

#[derive(Default, Debug, PartialEq, InitSpace, LightAccount)]
#[account]
#[repr(C)]
pub struct PoolState {
    pub compression_info: CompressionInfo,
    pub amm_config: Pubkey,
    pub pool_creator: Pubkey,
    pub token_0_vault: Pubkey,
    pub token_1_vault: Pubkey,
    pub lp_mint: Pubkey,
    pub token_0_mint: Pubkey,
    pub token_1_mint: Pubkey,
    pub token_0_program: Pubkey,
    pub token_1_program: Pubkey,
    pub observation_key: Pubkey,
    pub auth_bump: u8,
    pub status: u8,
    pub lp_mint_decimals: u8,
    pub mint_0_decimals: u8,
    pub mint_1_decimals: u8,
    pub lp_supply: u64,
    pub protocol_fees_token_0: u64,
    pub protocol_fees_token_1: u64,
    pub fund_fees_token_0: u64,
    pub fund_fees_token_1: u64,
    pub open_time: u64,
    pub recent_epoch: u64,
    pub padding: [u64; 1],
}

pub const OBSERVATION_NUM: usize = 2;

#[derive(Default, Clone, Copy, PartialEq, AnchorSerialize, AnchorDeserialize, InitSpace, Debug)]
pub struct Observation {
    pub block_timestamp: u64,
    pub cumulative_token_0_price_x32: u128,
    pub cumulative_token_1_price_x32: u128,
}

#[derive(Default, Debug, PartialEq, InitSpace, LightAccount)]
#[account]
pub struct ObservationState {
    pub compression_info: CompressionInfo,
    pub initialized: bool,
    pub observation_index: u16,
    pub pool_id: Pubkey,
    pub observations: [Observation; OBSERVATION_NUM],
    pub padding: [u64; 4],
}
