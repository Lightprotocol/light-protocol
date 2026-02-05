/*
 * photon-indexer
 *
 * Solana indexer for general compression
 *
 * The version of the OpenAPI document: 0.50.0
 *
 */

/// ResolvedFrom : Indicates the source of the resolved account data
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ResolvedFrom {
    #[serde(rename = "onchain")]
    Onchain,
    #[serde(rename = "compressed")]
    Compressed,
}

impl Default for ResolvedFrom {
    fn default() -> Self {
        Self::Onchain
    }
}
