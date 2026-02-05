/*
 * photon-indexer
 *
 * Solana indexer for general compression
 *
 * The version of the OpenAPI document: 0.50.0
 *
 */

/// GetAtaInterfacePostRequestParams : Request parameters for getAtaInterface
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct GetAtaInterfacePostRequestParams {
    /// Owner address
    #[serde(rename = "owner")]
    pub owner: String,
    /// Mint address
    #[serde(rename = "mint")]
    pub mint: String,
}

impl GetAtaInterfacePostRequestParams {
    pub fn new(owner: String, mint: String) -> Self {
        Self { owner, mint }
    }
}
