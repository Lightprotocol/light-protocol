use light_hasher::errors::HasherError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IndexedMerkleTreeError {
    #[error("Low element not found")]
    LowElementNotFound,
}
