use anchor_lang::prelude::*;

#[error_code]
pub enum AccountCompressionErrorCode {
    #[msg("Failed to insert an element into indexing queue")]
    AddressQueueInsert,
    #[msg("Failed to dequeue an element from indexing queue")]
    AddressQueueDequeue,
    #[msg("Failed to initialize address Merkle tree")]
    AddressMerkleTreeInitialize,
    #[msg("Failed to update the address Merkle tree")]
    AddressMerkleTreeUpdate,
    #[msg("No element found under the given index in the queue")]
    InvalidIndex,
    #[msg("Failed to convert bytes to big integer")]
    BytesToBigint,
    #[msg("Integer overflow")]
    IntegerOverflow,
}
