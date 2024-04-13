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
    #[error("Invalid height, it cannot exceed the maximum allowed height")]
    HeightHigherThanMax,
    #[error("Invalid number of roots, it has to be greater than 0")]
    RootsZero,
    #[error("Invalid root index, it exceeds the root buffer size")]
    RootHigherThanMax,
    #[error("Failed to read a value from bytes")]
    BytesRead,
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
    #[error("Cannot update tree without changelog, only `append` is supported.")]
    AppendOnly,
    #[error("The batch of leaves is empty")]
    EmptyLeaves,
    #[error("The vector of changelog entries is empty")]
    EmptyChangelogEntries,
    #[error(
        "Found an empty node in the Merkle path buffer, where we expected all nodes to be filled"
    )]
    MerklePathsEmptyNode,
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
            ConcurrentMerkleTreeError::IntegerOverflow => 2001,
            ConcurrentMerkleTreeError::HeightZero => 2002,
            ConcurrentMerkleTreeError::ChangelogZero => 2003,
            ConcurrentMerkleTreeError::HeightHigherThanMax => 2004,
            ConcurrentMerkleTreeError::RootsZero => 2005,
            ConcurrentMerkleTreeError::RootHigherThanMax => 2006,
            ConcurrentMerkleTreeError::BytesRead => 2007,
            ConcurrentMerkleTreeError::TreeFull => 2008,
            ConcurrentMerkleTreeError::BatchGreaterThanChangelog(_, _) => 2009,
            ConcurrentMerkleTreeError::InvalidProofLength(_, _) => 2010,
            ConcurrentMerkleTreeError::InvalidProof(_, _) => 2011,
            ConcurrentMerkleTreeError::CannotUpdateLeaf => 2012,
            ConcurrentMerkleTreeError::CannotUpdateEmpty => 2013,
            ConcurrentMerkleTreeError::AppendOnly => 2014,
            ConcurrentMerkleTreeError::EmptyLeaves => 2015,
            ConcurrentMerkleTreeError::EmptyChangelogEntries => 2016,
            ConcurrentMerkleTreeError::MerklePathsEmptyNode => 2017,
            ConcurrentMerkleTreeError::StructBufferSize(_, _) => 2018,
            ConcurrentMerkleTreeError::FilledSubtreesBufferSize(_, _) => 2019,
            ConcurrentMerkleTreeError::ChangelogBufferSize(_, _) => 2020,
            ConcurrentMerkleTreeError::RootBufferSize(_, _) => 2021,
            ConcurrentMerkleTreeError::CanopyBufferSize(_, _) => 2022,
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
