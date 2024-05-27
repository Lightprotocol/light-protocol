use light_bounded_vec::BoundedVecError;
use light_concurrent_merkle_tree::{
    errors::ConcurrentMerkleTreeError, light_hasher::errors::HasherError,
};
use light_utils::UtilsError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IndexedMerkleTreeError {
    #[error("Integer overflow")]
    IntegerOverflow,
    #[error("Invalid index, it exceeds the number of elements.")]
    IndexHigherThanMax,
    #[error("Could not find the low element.")]
    LowElementNotFound,
    #[error("Low element is greater or equal to the provided new element.")]
    LowElementGreaterOrEqualToNewElement,
    #[error("The provided new element is greater or equal to the next element.")]
    NewElementGreaterOrEqualToNextElement,
    #[error("The element already exists, but was expected to be absent.")]
    ElementAlreadyExists,
    #[error("The element does not exist, but was expected to be present.")]
    ElementDoesNotExist,
    #[error("Invalid changelog buffer size, expected {0}, got {1}")]
    ChangelogBufferSize(usize, usize),
    #[error("Hasher error: {0}")]
    Hasher(#[from] HasherError),
    #[error("Concurrent Merkle tree error: {0}")]
    ConcurrentMerkleTree(#[from] ConcurrentMerkleTreeError),
    #[error("Utils error {0}")]
    Utils(#[from] UtilsError),
    #[error("Bounded vector error: {0}")]
    BoundedVec(#[from] BoundedVecError),
    #[error("Next index of new element should be next index of the low element.")]
    NewElementNextIndexMismatch,
}

// NOTE(vadorovsky): Unfortunately, we need to do it by hand. `num_derive::ToPrimitive`
// doesn't support data-carrying enums.
#[cfg(feature = "solana")]
impl From<IndexedMerkleTreeError> for u32 {
    fn from(e: IndexedMerkleTreeError) -> u32 {
        match e {
            IndexedMerkleTreeError::IntegerOverflow => 11001,
            IndexedMerkleTreeError::IndexHigherThanMax => 11002,
            IndexedMerkleTreeError::LowElementNotFound => 11003,
            IndexedMerkleTreeError::LowElementGreaterOrEqualToNewElement => 11004,
            IndexedMerkleTreeError::NewElementGreaterOrEqualToNextElement => 11005,
            IndexedMerkleTreeError::ElementAlreadyExists => 11006,
            IndexedMerkleTreeError::ElementDoesNotExist => 11007,
            IndexedMerkleTreeError::ChangelogBufferSize(_, _) => 11008,
            IndexedMerkleTreeError::NewElementNextIndexMismatch => 11009,
            IndexedMerkleTreeError::Hasher(e) => e.into(),
            IndexedMerkleTreeError::ConcurrentMerkleTree(e) => e.into(),
            IndexedMerkleTreeError::Utils(e) => e.into(),
            IndexedMerkleTreeError::BoundedVec(e) => e.into(),
        }
    }
}

#[cfg(feature = "solana")]
impl From<IndexedMerkleTreeError> for solana_program::program_error::ProgramError {
    fn from(e: IndexedMerkleTreeError) -> Self {
        solana_program::program_error::ProgramError::Custom(e.into())
    }
}
