/*
 * photon-indexer
 *
 * Solana indexer for general compression
 *
 * The version of the OpenAPI document: 0.50.0
 *
 */

/// GetMultipleAccountInterfacesPostRequestParams : Request parameters for getMultipleAccountInterfaces
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct GetMultipleAccountInterfacesPostRequestParams {
    /// List of account addresses to look up (max 100)
    #[serde(rename = "addresses")]
    pub addresses: Vec<String>,
}

impl GetMultipleAccountInterfacesPostRequestParams {
    pub fn new(addresses: Vec<String>) -> Self {
        Self { addresses }
    }
}
