/*
 * photon-indexer
 *
 * Solana indexer for general compression
 *
 * The version of the OpenAPI document: 0.51.0
 *
 */

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[repr(u64)]
pub enum TreeType {
    #[default]
    #[serde(rename = "stateV1")]
    StateV1 = 1,
    #[serde(rename = "stateV2")]
    StateV2 = 3,
}

impl From<TreeType> for u64 {
    fn from(value: TreeType) -> Self {
        value as u64
    }
}

impl From<u64> for TreeType {
    fn from(value: u64) -> Self {
        match value {
            1 => TreeType::StateV1,
            3 => TreeType::StateV2,
            _ => TreeType::StateV1,
        }
    }
}

/// TreeInfo : Merkle tree info for compressed accounts
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeInfo {
    pub tree: String,
    pub queue: String,
    #[serde(rename = "treeType")]
    pub tree_type: TreeType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,
    /// Slot when the account was created/compressed
    #[serde(rename = "slotCreated")]
    pub slot_created: u64,
}

impl TreeInfo {
    pub fn new(tree: String, queue: String, tree_type: TreeType, slot_created: u64) -> Self {
        Self {
            tree,
            queue,
            tree_type,
            seq: None,
            slot_created,
        }
    }
}
