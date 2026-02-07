#[derive(Debug, Clone, PartialEq, Default)]
pub struct SignatureWithMetadata {
    pub block_time: u64,
    pub signature: String,
    pub slot: u64,
}

impl From<&photon_api::types::SignatureInfo> for SignatureWithMetadata {
    fn from(sig_info: &photon_api::types::SignatureInfo) -> Self {
        SignatureWithMetadata {
            block_time: *sig_info.block_time as u64,
            signature: sig_info.signature.0.clone(),
            slot: *sig_info.slot,
        }
    }
}
