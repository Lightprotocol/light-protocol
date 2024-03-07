use std::fmt;

#[derive(Clone, Debug)]
pub enum NonInclusionMerkleTreeInfo {
    H26,
}

// TODO: split inclusion & non-inclusion
impl NonInclusionMerkleTreeInfo {
    pub fn height(&self) -> u8 {
        match self {
            NonInclusionMerkleTreeInfo::H26 => 26,
        }
    }

    pub fn test_zk_path(&self, num_of_utxos: usize) -> String {
        format!(
            "test-data/ni_{}_{}/ni_{}_{}.zkey",
            self.height(),
            num_of_utxos,
            self.height(),
            num_of_utxos
        )
    }
    pub fn test_wasm_path(&self, num_of_utxos: usize) -> String {
        format!(
            "test-data/ni_{}_{}/ni_{}_{}.wasm",
            self.height(),
            num_of_utxos,
            self.height(),
            num_of_utxos
        )
    }
}

impl fmt::Display for NonInclusionMerkleTreeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.height())
    }
}
