/*
 * photon-indexer
 *
 * Solana indexer for general compression
 *
 * The version of the OpenAPI document: 0.50.0
 *
 */

use crate::models;

/// InterfaceResult : Heterogeneous result type for batch lookups
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum InterfaceResult {
    /// Generic account result
    #[serde(rename = "account")]
    Account(models::AccountInterface),
    /// Token account result with parsed token data
    #[serde(rename = "token")]
    Token(models::TokenAccountInterface),
}

impl Default for InterfaceResult {
    fn default() -> Self {
        Self::Account(models::AccountInterface::default())
    }
}
