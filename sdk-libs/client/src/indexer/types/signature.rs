use super::super::IndexerError;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SignatureWithMetadata {
    pub block_time: u64,
    pub signature: String,
    pub slot: u64,
}

impl TryFrom<&photon_api::models::SignatureInfo> for SignatureWithMetadata {
    type Error = IndexerError;

    fn try_from(sig_info: &photon_api::models::SignatureInfo) -> Result<Self, Self::Error> {
        Ok(SignatureWithMetadata {
            block_time: sig_info.block_time,
            signature: sig_info.signature.clone(),
            slot: sig_info.slot,
        })
    }
}
