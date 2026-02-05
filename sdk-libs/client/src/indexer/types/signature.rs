#[derive(Debug, Clone, PartialEq, Default)]
pub struct SignatureWithMetadata {
    pub block_time: u64,
    pub signature: String,
    pub slot: u64,
}

impl From<&photon_api::models::SignatureInfo> for SignatureWithMetadata {
    fn from(sig_info: &photon_api::models::SignatureInfo) -> Self {
        SignatureWithMetadata {
            block_time: sig_info.block_time,
            signature: sig_info.signature.clone(),
            slot: sig_info.slot,
        }
    }
}
