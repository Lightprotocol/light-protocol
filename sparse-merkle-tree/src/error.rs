use light_hasher::HasherError;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum SparseMerkleTreeError {
    #[error("Update proof cannot update leaf from the changelog.")]
    CannotUpdateLeaf,
    #[error("Hasher error: {0}")]
    Hasher(#[from] HasherError),
}
