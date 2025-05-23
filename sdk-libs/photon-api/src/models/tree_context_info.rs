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
pub struct TreeContextInfo {
    /// A Solana public key represented as a base58 string.
    #[serde(rename = "cpiContext", skip_serializing_if = "Option::is_none")]
    pub cpi_context: Option<String>,
    /// A Solana public key represented as a base58 string.
    #[serde(rename = "queue")]
    pub queue: String,
    /// A Solana public key represented as a base58 string.
    #[serde(rename = "tree")]
    pub tree: String,
    #[serde(rename = "treeType")]
    pub tree_type: u16,
}

impl TreeContextInfo {
    pub fn new(queue: String, tree: String, tree_type: u16) -> TreeContextInfo {
        TreeContextInfo {
            cpi_context: None,
            queue,
            tree,
            tree_type,
        }
    }
}
