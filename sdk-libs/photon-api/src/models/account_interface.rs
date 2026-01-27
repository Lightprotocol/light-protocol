/*
 * photon-indexer
 *
 * Solana indexer for general compression
 *
 * The version of the OpenAPI document: 0.50.0
 *
 */

use crate::models;

/// AccountInterface : Unified account interface representing either on-chain or compressed account data
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccountInterface {
    /// The account address (pubkey for on-chain, compressed address for compressed)
    #[serde(rename = "address")]
    pub address: String,
    /// Account lamports balance
    #[serde(rename = "lamports")]
    pub lamports: u64,
    /// The program owner of this account
    #[serde(rename = "owner")]
    pub owner: String,
    /// Account data as base64 encoded bytes
    #[serde(rename = "data")]
    pub data: String,
    /// Whether the account is executable (always false for compressed)
    #[serde(rename = "executable")]
    pub executable: bool,
    /// Rent epoch (always 0 for compressed)
    #[serde(rename = "rentEpoch")]
    pub rent_epoch: u64,
    /// Source of the account data
    #[serde(rename = "resolvedFrom")]
    pub resolved_from: models::ResolvedFrom,
    /// Slot at which the account data was resolved
    #[serde(rename = "resolvedSlot")]
    pub resolved_slot: u64,
    /// Additional context for compressed accounts (None for on-chain)
    #[serde(rename = "compressedContext", skip_serializing_if = "Option::is_none")]
    pub compressed_context: Option<Box<models::CompressedContext>>,
}

impl AccountInterface {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        address: String,
        lamports: u64,
        owner: String,
        data: String,
        executable: bool,
        rent_epoch: u64,
        resolved_from: models::ResolvedFrom,
        resolved_slot: u64,
    ) -> Self {
        Self {
            address,
            lamports,
            owner,
            data,
            executable,
            rent_epoch,
            resolved_from,
            resolved_slot,
            compressed_context: None,
        }
    }
}
