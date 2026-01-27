/*
 * photon-indexer
 *
 * Solana indexer for general compression
 *
 * The version of the OpenAPI document: 0.50.0
 *
 */

/// CompressedContext : Context information for compressed accounts
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompressedContext {
    /// The hash of the compressed account (leaf hash in Merkle tree)
    #[serde(rename = "hash")]
    pub hash: String,
    /// The Merkle tree address
    #[serde(rename = "tree")]
    pub tree: String,
    /// The leaf index in the Merkle tree
    #[serde(rename = "leafIndex")]
    pub leaf_index: u64,
    /// Sequence number (None if in output queue, Some once inserted into Merkle tree)
    #[serde(rename = "seq", skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,
    /// Whether the account can be proven by index (in output queue)
    #[serde(rename = "proveByIndex")]
    pub prove_by_index: bool,
}

impl CompressedContext {
    pub fn new(hash: String, tree: String, leaf_index: u64, prove_by_index: bool) -> Self {
        Self {
            hash,
            tree,
            leaf_index,
            seq: None,
            prove_by_index,
        }
    }
}
