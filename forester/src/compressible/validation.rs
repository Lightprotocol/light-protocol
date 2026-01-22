//! Startup validation for compressible configurations.
//!
//! This module provides functions to validate on-chain configuration accounts
//! at forester startup, allowing fail-fast behavior on misconfigurations.

use std::str::FromStr;

use anchor_lang::AccountDeserialize;
use light_compressible::config::CompressibleConfig as OnChainCompressibleConfig;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

use super::config::REGISTRY_PROGRAM_ID;
use crate::Result;

/// Validates the on-chain CompressibleConfig for CToken/Mint compression.
///
/// Fetches the CompressibleConfig PDA from the registry program and validates:
/// - Account exists
/// - State is not Inactive (0 = paused)
///
/// Active (1) and Deprecated (2) states are both valid for compression operations.
///
/// # Errors
///
/// Returns an error if:
/// - The config account doesn't exist
/// - The config state is Inactive (paused)
/// - RPC communication fails
pub async fn validate_compressible_config(rpc_url: &str) -> Result<()> {
    let registry_program_id = Pubkey::from_str(REGISTRY_PROGRAM_ID)?;

    // Derive the CompressibleConfig PDA
    let (config_pda, _) = OnChainCompressibleConfig::derive_v1_config_pda(&registry_program_id);

    // Fetch the account
    let rpc_client = RpcClient::new(rpc_url.to_string());
    let account = rpc_client.get_account(&config_pda).await.map_err(|e| {
        anyhow::anyhow!(
            "Failed to fetch CompressibleConfig at {}: {}",
            config_pda,
            e
        )
    })?;

    // Deserialize using AccountDeserialize to validate the discriminator
    let config = OnChainCompressibleConfig::try_deserialize(&mut account.data.as_slice())
        .map_err(|e| anyhow::anyhow!("Failed to deserialize CompressibleConfig: {:?}", e))?;

    // Validate state is not Inactive (0)
    // State values: 0 = Inactive (paused), 1 = Active, 2 = Deprecated
    // Both Active and Deprecated are valid for compression operations
    config.validate_not_inactive().map_err(|e| {
        anyhow::anyhow!(
            "CompressibleConfig validation failed: {:?}. Config PDA: {}",
            e,
            config_pda
        )
    })?;

    tracing::info!(
        "CompressibleConfig validated: PDA={}, state={} ({})",
        config_pda,
        config.state,
        match config.state {
            1 => "Active",
            2 => "Deprecated",
            _ => "Unknown",
        }
    );

    Ok(())
}
