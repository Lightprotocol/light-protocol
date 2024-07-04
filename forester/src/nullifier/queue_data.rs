use light_test_utils::indexer::NewAddressProofWithContext;

#[derive(Debug, Clone)]
pub struct ForesterQueueAccountData {
    pub account: ForesterQueueAccount,
    pub proof: Vec<[u8; 32]>,
    pub leaf_index: u64,
    pub root_seq: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct ForesterQueueAccount {
    pub hash: [u8; 32],
    pub index: usize,
}

impl ForesterQueueAccount {
    pub fn hash_string(&self) -> String {
        bs58::encode(&self.hash).into_string()
    }
}

#[derive(Debug)]
pub struct ForesterAddressQueueAccountData {
    pub account: ForesterQueueAccount,
    pub proof: NewAddressProofWithContext

}
