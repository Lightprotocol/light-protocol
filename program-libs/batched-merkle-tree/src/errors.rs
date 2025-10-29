use light_account_checks::error::AccountError;
use light_bloom_filter::BloomFilterError;
use light_compressed_account::CompressedAccountError;
use light_hasher::HasherError;
use light_merkle_tree_metadata::errors::MerkleTreeMetadataError;
use light_verifier::VerifierError;
use light_zero_copy::errors::ZeroCopyError;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum BatchedMerkleTreeError {
    #[error("Batch is not ready to be inserted")]
    BatchNotReady,
    #[error("Batch is already inserted")]
    BatchAlreadyInserted,
    #[error("Batch insert failed")]
    BatchInsertFailed,
    #[error("Leaf index not in batch.")]
    LeafIndexNotInBatch,
    #[error("Invalid network fee.")]
    InvalidNetworkFee,
    #[error("Batch size not divisible by ZKP batch size.")]
    BatchSizeNotDivisibleByZkpBatchSize,
    #[error("Inclusion proof by index failed.")]
    InclusionProofByIndexFailed,
    #[error("Hasher error: {0}")]
    Hasher(#[from] HasherError),
    #[error("Compressed Account error {0}")]
    CompressedAccountError(#[from] CompressedAccountError),
    #[error("Zero copy error {0}")]
    ZeroCopy(#[from] ZeroCopyError),
    #[error("Merkle tree metadata error {0}")]
    MerkleTreeMetadata(#[from] MerkleTreeMetadataError),
    #[error("Bloom filter error {0}")]
    BloomFilter(#[from] BloomFilterError),
    #[cfg(feature = "pinocchio")]
    #[error("Program error {0}")]
    ProgramError(u64),
    #[cfg(all(feature = "solana", not(feature = "pinocchio")))]
    #[error("Program error {0}")]
    ProgramError(#[from] solana_program_error::ProgramError),
    #[error("Verifier error {0}")]
    VerifierErrorError(#[from] VerifierError),
    #[error("Invalid batch index")]
    InvalidBatchIndex,
    #[error("Invalid index")]
    InvalidIndex,
    #[error("Batched Merkle tree is full.")]
    TreeIsFull,
    #[error("Value already exists in bloom filter.")]
    NonInclusionCheckFailed,
    #[error("Bloom filter must be zeroed prior to reusing a batch.")]
    BloomFilterNotZeroed,
    #[error("Cannot zero out complete or more than complete root history.")]
    CannotZeroCompleteRootHistory,
    #[error("Account error {0}")]
    AccountError(#[from] AccountError),
}

impl From<BatchedMerkleTreeError> for u32 {
    fn from(e: BatchedMerkleTreeError) -> u32 {
        match e {
            BatchedMerkleTreeError::BatchNotReady => 14301,
            BatchedMerkleTreeError::BatchAlreadyInserted => 14302,
            BatchedMerkleTreeError::BatchInsertFailed => 14303,
            BatchedMerkleTreeError::LeafIndexNotInBatch => 14304,
            BatchedMerkleTreeError::InvalidNetworkFee => 14305,
            BatchedMerkleTreeError::BatchSizeNotDivisibleByZkpBatchSize => 14306,
            BatchedMerkleTreeError::InclusionProofByIndexFailed => 14307,
            BatchedMerkleTreeError::InvalidBatchIndex => 14308,
            BatchedMerkleTreeError::InvalidIndex => 14309,
            BatchedMerkleTreeError::TreeIsFull => 14310,
            BatchedMerkleTreeError::NonInclusionCheckFailed => 14311,
            BatchedMerkleTreeError::BloomFilterNotZeroed => 14312,
            BatchedMerkleTreeError::CannotZeroCompleteRootHistory => 14313,
            BatchedMerkleTreeError::Hasher(e) => e.into(),
            BatchedMerkleTreeError::ZeroCopy(e) => e.into(),
            BatchedMerkleTreeError::MerkleTreeMetadata(e) => e.into(),
            BatchedMerkleTreeError::BloomFilter(e) => e.into(),
            BatchedMerkleTreeError::VerifierErrorError(e) => e.into(),
            BatchedMerkleTreeError::CompressedAccountError(e) => e.into(),
            #[cfg(any(feature = "pinocchio", feature = "solana"))]
            #[allow(clippy::useless_conversion)]
            BatchedMerkleTreeError::ProgramError(e) => u32::try_from(u64::from(e)).unwrap(),
            BatchedMerkleTreeError::AccountError(e) => e.into(),
        }
    }
}

#[cfg(feature = "solana")]
impl From<BatchedMerkleTreeError> for solana_program_error::ProgramError {
    fn from(e: BatchedMerkleTreeError) -> Self {
        solana_program_error::ProgramError::Custom(e.into())
    }
}

#[cfg(feature = "pinocchio")]
impl From<BatchedMerkleTreeError> for pinocchio::program_error::ProgramError {
    fn from(e: BatchedMerkleTreeError) -> Self {
        pinocchio::program_error::ProgramError::Custom(e.into())
    }
}

#[cfg(feature = "pinocchio")]
impl From<pinocchio::program_error::ProgramError> for BatchedMerkleTreeError {
    fn from(error: pinocchio::program_error::ProgramError) -> Self {
        BatchedMerkleTreeError::ProgramError(u64::from(error))
    }
}
