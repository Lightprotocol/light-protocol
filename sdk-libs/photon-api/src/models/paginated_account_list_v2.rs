/*
 * photon-indexer
 *
 * Solana indexer for general compression
 *
 * The version of the OpenAPI document: 0.50.0
 *
 * Generated by: https://openapi-generator.tech
 */

use crate::models;

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct PaginatedAccountListV2 {
    /// A 32-byte hash represented as a base58 string.
    #[serde(rename = "cursor", skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(rename = "items")]
    pub items: Vec<models::AccountV2>,
}

impl PaginatedAccountListV2 {
    pub fn new(items: Vec<models::AccountV2>) -> PaginatedAccountListV2 {
        PaginatedAccountListV2 {
            cursor: None,
            items,
        }
    }
}
