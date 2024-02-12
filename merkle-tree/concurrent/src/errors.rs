use light_hasher::errors::HasherError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConcurrentMerkleTreeError {
    #[error("Invalid height, it has to be greater than 0")]
    HeightZero,
    #[error("Invalid height, it cannot exceed the maximum allowed height")]
    HeightHigherThanMax,
    #[error("Invalid number of roots, it has to be greater than 0")]
    RootsZero,
    #[error("Invalid root index, it exceeds the root buffer size")]
    RootHigherThanMax,
    #[error("Merkle tree is full, cannot append more leaves.")]
    TreeFull,
    #[error("Provided proof is larger than the height of the tree.")]
    ProofTooLarge,
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
    #[error("Hasher error: {0}")]
    Hasher(#[from] HasherError),
}

// NOTE(vadorovsky): Unfortunately, we need to do it by hand. `num_derive::ToPrimitive`
// doesn't support data-carrying enums.
#[cfg(feature = "solana")]
impl From<ConcurrentMerkleTreeError> for u32 {
    fn from(e: ConcurrentMerkleTreeError) -> u32 {
        match e {
            ConcurrentMerkleTreeError::HeightZero => 2001,
            ConcurrentMerkleTreeError::HeightHigherThanMax => 2002,
            ConcurrentMerkleTreeError::RootsZero => 2003,
            ConcurrentMerkleTreeError::RootHigherThanMax => 2004,
            ConcurrentMerkleTreeError::TreeFull => 2005,
            ConcurrentMerkleTreeError::ProofTooLarge => 2006,
            ConcurrentMerkleTreeError::InvalidProof(_, _) => 2007,
            ConcurrentMerkleTreeError::CannotUpdateLeaf => 2008,
            ConcurrentMerkleTreeError::CannotUpdateEmpty => 2009,
            ConcurrentMerkleTreeError::AppendOnly => 2010,
            ConcurrentMerkleTreeError::EmptyLeaves => 2011,
            ConcurrentMerkleTreeError::EmptyChangelogEntries => 2012,
            ConcurrentMerkleTreeError::MerklePathsEmptyNode => 2013,
            ConcurrentMerkleTreeError::Hasher(e) => e.into(),
        }
    }
}

#[cfg(feature = "solana")]
impl From<ConcurrentMerkleTreeError> for solana_program::program_error::ProgramError {
    fn from(e: ConcurrentMerkleTreeError) -> Self {
        solana_program::program_error::ProgramError::Custom(e.into())
    }
}
