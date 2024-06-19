#[derive(Debug)]
pub struct QueueData {
    pub(crate) accounts_to_nullify: Vec<AccountData>,
}

impl QueueData {
    pub(crate) fn new(accounts_to_nullify: Vec<AccountData>) -> Self {
        Self {
            accounts_to_nullify,
        }
    }
}

#[derive(Debug)]
pub struct AccountData {
    pub account: Account,
    pub proof: Vec<[u8; 32]>,
    pub leaf_index: u64,
    pub root_seq: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct Account {
    pub hash: [u8; 32],
    pub index: usize,
}

impl Account {
    pub fn hash_string(&self) -> String {
        bs58::encode(&self.hash).into_string()
    }
}
