/*
 * photon-indexer
 *
 * Solana indexer for general compression
 *
 * The version of the OpenAPI document: 0.51.0
 *
 */

use crate::models;

/// ColdContext : Compressed account context — present when account is in compressed state
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ColdContext {
    /// Generic compressed account
    #[serde(rename = "account")]
    Account {
        hash: String,
        #[serde(rename = "leafIndex")]
        leaf_index: u64,
        #[serde(rename = "treeInfo")]
        tree_info: models::InterfaceTreeInfo,
        data: models::ColdData,
    },
    /// Compressed token account
    #[serde(rename = "token")]
    Token {
        hash: String,
        #[serde(rename = "leafIndex")]
        leaf_index: u64,
        #[serde(rename = "treeInfo")]
        tree_info: models::InterfaceTreeInfo,
        data: models::ColdData,
    },
}
