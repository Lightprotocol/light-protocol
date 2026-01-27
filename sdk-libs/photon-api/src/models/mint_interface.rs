/*
 * photon-indexer
 *
 * Solana indexer for general compression
 *
 * The version of the OpenAPI document: 0.50.0
 *
 */

use crate::models;

/// MintInterface : Mint account interface with parsed mint data
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct MintInterface {
    /// Base account interface data (flattened)
    #[serde(flatten)]
    pub account: models::AccountInterface,
    /// Parsed mint data
    #[serde(rename = "mintData")]
    pub mint_data: models::MintData,
}

impl MintInterface {
    pub fn new(account: models::AccountInterface, mint_data: models::MintData) -> Self {
        Self { account, mint_data }
    }
}
