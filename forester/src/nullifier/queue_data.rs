use std::collections::HashMap;

pub struct QueueData {
    pub change_log_index: usize,
    pub sequence_number: usize,
    pub compressed_accounts_to_nullify: Vec<Account>,
    pub compressed_account_proofs: HashMap<String, (Vec<[u8; 32]>, u64, i64)>,
}

#[derive(Clone, Copy)]
pub struct Account {
    pub hash: [u8; 32],
    pub index: usize,
}

impl Account {
    pub fn hash_string(&self) -> String {
        bs58::encode(&self.hash).into_string()
    }
}
