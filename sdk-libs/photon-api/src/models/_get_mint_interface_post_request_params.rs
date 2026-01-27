/*
 * photon-indexer
 *
 * Solana indexer for general compression
 *
 * The version of the OpenAPI document: 0.50.0
 *
 */

/// GetMintInterfacePostRequestParams : Request parameters for getMintInterface
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct GetMintInterfacePostRequestParams {
    /// Mint address to look up
    #[serde(rename = "address")]
    pub address: String,
}

impl GetMintInterfacePostRequestParams {
    pub fn new(address: String) -> Self {
        Self { address }
    }
}
