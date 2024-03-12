use std::fmt;

#[derive(Clone, Debug)]
pub enum MerkleTreeInfo {
    H26,
}

impl MerkleTreeInfo {
    pub fn height(&self) -> u8 {
        match self {
            MerkleTreeInfo::H26 => 26,
        }
    }

    pub fn test_zk_path(&self, num_of_utxos: usize) -> String {
        format!(
            "test-data/i_{}_{}/i_{}_{}.zkey",
            self.height(),
            num_of_utxos,
            self.height(),
            num_of_utxos
        )
    }
    pub fn test_wasm_path(&self, num_of_utxos: usize) -> String {
        format!(
            "test-data/i_{}_{}/i_{}_{}.wasm",
            self.height(),
            num_of_utxos,
            self.height(),
            num_of_utxos
        )
    }
}

impl fmt::Display for MerkleTreeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.height())
    }
}
