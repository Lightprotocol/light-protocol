use light_bounded_vec::BoundedVecError;
use light_hasher::errors::HasherError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConcurrentMerkleTreeError {
    #[error("Integer overflow")]
    IntegerOverflow,
    #[error("Invalid height, it has to be greater than 0")]
    HeightZero,
    #[error("Invalid changelog size, it has to be greater than 0. Changelog is used for storing Merkle paths during appends.")]
    ChangelogZero,
    #[error("Invalid number of roots, it has to be greater than 0")]
    RootsZero,
    #[error("Merkle tree is full, cannot append more leaves.")]
    TreeFull,
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
    #[error("Invalid struct buffer size, expected {0}, got {1}")]
    StructBufferSize(usize, usize),
    #[error("Invalid filled subtrees buffer size, expected {0}, got {1}")]
    FilledSubtreesBufferSize(usize, usize),
    #[error("Invalid changelog buffer size, expected {0}, got {1}")]
    ChangelogBufferSize(usize, usize),
    #[error("Invalid root buffer size, expected {0}, got {1}")]
    RootBufferSize(usize, usize),
    #[error("Invalid canopy buffer size, expected {0}, got {1}")]
    CanopyBufferSize(usize, usize),
    #[error("Hasher error: {0}")]
    Hasher(#[from] HasherError),
    #[error("Bounded vector error: {0}")]
    BoundedVec(#[from] BoundedVecError),
}

// NOTE(vadorovsky): Unfortunately, we need to do it by hand. `num_derive::ToPrimitive`
// doesn't support data-carrying enums.
#[cfg(feature = "solana")]
impl From<ConcurrentMerkleTreeError> for u32 {
    fn from(e: ConcurrentMerkleTreeError) -> u32 {
        match e {
            ConcurrentMerkleTreeError::IntegerOverflow => 10001,
            ConcurrentMerkleTreeError::HeightZero => 10002,
            ConcurrentMerkleTreeError::ChangelogZero => 10003,
            ConcurrentMerkleTreeError::RootsZero => 10004,
            ConcurrentMerkleTreeError::TreeFull => 10005,
            ConcurrentMerkleTreeError::BatchGreaterThanChangelog(_, _) => 10006,
            ConcurrentMerkleTreeError::InvalidProofLength(_, _) => 10007,
            ConcurrentMerkleTreeError::InvalidProof(_, _) => 10008,
            ConcurrentMerkleTreeError::CannotUpdateLeaf => 10009,
            ConcurrentMerkleTreeError::CannotUpdateEmpty => 10010,
            ConcurrentMerkleTreeError::EmptyLeaves => 10011,
            ConcurrentMerkleTreeError::StructBufferSize(_, _) => 10012,
            ConcurrentMerkleTreeError::FilledSubtreesBufferSize(_, _) => 10013,
            ConcurrentMerkleTreeError::ChangelogBufferSize(_, _) => 100014,
            ConcurrentMerkleTreeError::RootBufferSize(_, _) => 10015,
            ConcurrentMerkleTreeError::CanopyBufferSize(_, _) => 10016,
            ConcurrentMerkleTreeError::Hasher(e) => e.into(),
            ConcurrentMerkleTreeError::BoundedVec(e) => e.into(),
        }
    }
}

#[cfg(feature = "solana")]
impl From<ConcurrentMerkleTreeError> for solana_program::program_error::ProgramError {
    fn from(e: ConcurrentMerkleTreeError) -> Self {
        solana_program::program_error::ProgramError::Custom(e.into())
    }
}
