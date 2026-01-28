/*
 * photon-indexer
 *
 * Solana indexer for general compression
 *
 * The version of the OpenAPI document: 0.51.0
 *
 */

/// SolanaAccountData : Standard Solana account fields (matches getAccountInfo shape)
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolanaAccountData {
    pub lamports: u64,
    pub data: String,
    pub owner: String,
    pub executable: bool,
    pub rent_epoch: u64,
}

impl SolanaAccountData {
    pub fn new(
        lamports: u64,
        data: String,
        owner: String,
        executable: bool,
        rent_epoch: u64,
    ) -> Self {
        Self {
            lamports,
            data,
            owner,
            executable,
            rent_epoch,
        }
    }
}
