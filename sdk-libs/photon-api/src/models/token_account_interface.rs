/*
 * photon-indexer
 *
 * Solana indexer for general compression
 *
 * The version of the OpenAPI document: 0.50.0
 *
 */

use crate::models;

/// TokenAccountInterface : Token account interface with parsed token data
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenAccountInterface {
    /// Base account interface data (flattened)
    #[serde(flatten)]
    pub account: models::AccountInterface,
    /// Parsed token account data
    #[serde(rename = "tokenData")]
    pub token_data: models::TokenData,
}

impl TokenAccountInterface {
    pub fn new(account: models::AccountInterface, token_data: models::TokenData) -> Self {
        Self {
            account,
            token_data,
        }
    }
}
