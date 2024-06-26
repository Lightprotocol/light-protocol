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
