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
pub struct ContextInfo {
    /// A Solana public key represented as a base58 string.
    #[serde(rename = "cpiContext", skip_serializing_if = "Option::is_none")]
    pub cpi_context: Option<String>,
    /// A Solana public key represented as a base58 string.
    #[serde(rename = "merkleTree")]
    pub merkle_tree: String,
    /// A Solana public key represented as a base58 string.
    #[serde(rename = "queue")]
    pub queue: String,
    #[serde(rename = "treeType")]
    pub tree_type: i32,
}

impl ContextInfo {
    pub fn new(merkle_tree: String, queue: String, tree_type: i32) -> ContextInfo {
        ContextInfo {
            cpi_context: None,
            merkle_tree,
            queue,
            tree_type,
        }
    }
}
