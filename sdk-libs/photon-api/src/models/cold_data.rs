/*
 * photon-indexer
 *
 * Solana indexer for general compression
 *
 * The version of the OpenAPI document: 0.51.0
 *
 */

/// ColdData : Structured compressed account data (discriminator separated)
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColdData {
    /// First 8 bytes of the account data (discriminator)
    pub discriminator: [u8; 8],
    /// Remaining account data after discriminator, base64 encoded
    pub data: String,
}

impl ColdData {
    pub fn new(discriminator: [u8; 8], data: String) -> Self {
        Self {
            discriminator,
            data,
        }
    }
}
