/*
 * photon-indexer
 *
 * Solana indexer for general compression
 *
 * The version of the OpenAPI document: 0.51.0
 *
 */

use crate::models;

/// AccountInterface : Unified account interface — works for both on-chain and compressed accounts
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInterface {
    /// The on-chain Solana pubkey
    #[serde(rename = "key")]
    pub key: String,
    /// Standard Solana account fields
    #[serde(rename = "account")]
    pub account: models::SolanaAccountData,
    /// Compressed context — null if on-chain, present if compressed
    #[serde(rename = "cold", skip_serializing_if = "Option::is_none")]
    pub cold: Option<models::ColdContext>,
}

impl AccountInterface {
    pub fn new(key: String, account: models::SolanaAccountData) -> Self {
        Self {
            key,
            account,
            cold: None,
        }
    }
}
