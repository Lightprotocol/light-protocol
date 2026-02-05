/*
 * photon-indexer
 *
 * Solana indexer for general compression
 *
 * The version of the OpenAPI document: 0.50.0
 *
 */

/// GetAccountInterfacePostRequestParams : Request parameters for getAccountInterface
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct GetAccountInterfacePostRequestParams {
    /// Account address to look up
    #[serde(rename = "address")]
    pub address: String,
}

impl GetAccountInterfacePostRequestParams {
    pub fn new(address: String) -> Self {
        Self { address }
    }
}
