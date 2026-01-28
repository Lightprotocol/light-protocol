/*
 * photon-indexer
 *
 * Solana indexer for general compression
 *
 * The version of the OpenAPI document: 0.51.0
 *
 */

/// TreeInfo : Merkle tree info for compressed accounts
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeInfo {
    pub tree: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,
}

impl TreeInfo {
    pub fn new(tree: String) -> Self {
        Self { tree, seq: None }
    }
}
