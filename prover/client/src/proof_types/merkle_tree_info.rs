use std::fmt;

#[derive(Clone, Debug)]
pub enum MerkleTreeInfo {
    H26,
    H32,
}

impl MerkleTreeInfo {
    pub fn height(&self) -> u8 {
        match self {
            MerkleTreeInfo::H26 => 26,
            MerkleTreeInfo::H32 => 32,
        }
    }
}

impl fmt::Display for MerkleTreeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.height())
    }
}
