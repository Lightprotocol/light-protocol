use light_bounded_vec::BoundedVecError;
use light_hasher::errors::HasherError;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum ConcurrentMerkleTreeError {
    #[error("Integer overflow")]
    IntegerOverflow,
    #[error("Invalid height, it has to be greater than 0")]
    HeightZero,
    #[error("Invalud height, expected {0}")]
    InvalidHeight(usize),
    #[error("Invalid changelog size, it has to be greater than 0. Changelog is used for storing Merkle paths during appends.")]
    ChangelogZero,
    #[error("Invalid number of roots, it has to be greater than 0")]
    RootsZero,
    #[error("Canopy depth has to be lower than height")]
    CanopyGeThanHeight,
    #[error("Merkle tree is full, cannot append more leaves.")]
    TreeIsFull,
    #[error("Number of leaves ({0}) exceeds the changelog capacity ({1}).")]
    BatchGreaterThanChangelog(usize, usize),
    #[error("Invalid proof length, expected {0}, got {1}.")]
    InvalidProofLength(usize, usize),
    #[error("Invalid Merkle proof, expected root: {0:?}, the provided proof produces root: {1:?}")]
    InvalidProof([u8; 32], [u8; 32]),
    #[error("Attempting to update the leaf which was updated by an another newest change.")]
    CannotUpdateLeaf,
    #[error("Cannot update the empty leaf")]
    CannotUpdateEmpty,
    #[error("The batch of leaves is empty")]
    EmptyLeaves,
    #[error("Invalid buffer size, expected {0}, got {1}")]
    BufferSize(usize, usize),
    #[error("Hasher error: {0}")]
    Hasher(#[from] HasherError),
    #[error("Bounded vector error: {0}")]
    BoundedVec(#[from] BoundedVecError),
}

impl From<ConcurrentMerkleTreeError> for u32 {
    fn from(e: ConcurrentMerkleTreeError) -> u32 {
        match e {
            ConcurrentMerkleTreeError::IntegerOverflow => 10001,
            ConcurrentMerkleTreeError::HeightZero => 10002,
            ConcurrentMerkleTreeError::InvalidHeight(_) => 10003,
            ConcurrentMerkleTreeError::ChangelogZero => 10004,
            ConcurrentMerkleTreeError::RootsZero => 10005,
            ConcurrentMerkleTreeError::CanopyGeThanHeight => 10006,
            ConcurrentMerkleTreeError::TreeIsFull => 10007,
            ConcurrentMerkleTreeError::BatchGreaterThanChangelog(_, _) => 10008,
            ConcurrentMerkleTreeError::InvalidProofLength(_, _) => 10009,
            ConcurrentMerkleTreeError::InvalidProof(_, _) => 10010,
            ConcurrentMerkleTreeError::CannotUpdateLeaf => 10011,
            ConcurrentMerkleTreeError::CannotUpdateEmpty => 10012,
            ConcurrentMerkleTreeError::EmptyLeaves => 10013,
            ConcurrentMerkleTreeError::BufferSize(_, _) => 10014,
            ConcurrentMerkleTreeError::Hasher(e) => e.into(),
            ConcurrentMerkleTreeError::BoundedVec(e) => e.into(),
        }
    }
}

#[cfg(feature = "solana")]
impl From<ConcurrentMerkleTreeError> for solana_program_error::ProgramError {
    fn from(e: ConcurrentMerkleTreeError) -> Self {
        solana_program_error::ProgramError::Custom(e.into())
    }
}

#[cfg(feature = "pinocchio")]
impl From<ConcurrentMerkleTreeError> for pinocchio::program_error::ProgramError {
    fn from(e: ConcurrentMerkleTreeError) -> Self {
        pinocchio::program_error::ProgramError::Custom(e.into())
    }
}
