/*
 * photon-indexer
 *
 * Solana indexer for general compression
 *
 * The version of the OpenAPI document: 0.50.0
 *
 */

/// GetTokenAccountInterfacePostRequestParams : Request parameters for getTokenAccountInterface
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct GetTokenAccountInterfacePostRequestParams {
    /// Token account address to look up
    #[serde(rename = "address")]
    pub address: String,
}

impl GetTokenAccountInterfacePostRequestParams {
    pub fn new(address: String) -> Self {
        Self { address }
    }
}
