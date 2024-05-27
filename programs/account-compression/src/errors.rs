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
    #[msg("InvalidAuthority")]
    InvalidAuthority,
    #[msg("InvalidVerifier")]
    InvalidVerifier,
    #[msg(
        "Leaves <> remaining accounts mismatch. The number of remaining accounts must match the number of leaves."
    )]
    NumberOfLeavesMismatch,
    #[msg("Provided noop program public key is invalid")]
    InvalidNoopPubkey,
    #[msg("Emitting an event requires at least one changelog entry")]
    EventNoChangelogEntry,
    #[msg("Number of change log indices mismatch")]
    NumberOfChangeLogIndicesMismatch,
    #[msg("Number of indices mismatch")]
    NumberOfIndicesMismatch,
    #[msg("IndexOutOfBounds")]
    IndexOutOfBounds,
    #[msg("NumberOfProofsMismatch")]
    NumberOfProofsMismatch,
    #[msg("InvalidMerkleProof")]
    InvalidMerkleProof,
    #[msg("InvalidQueue")]
    InvalidQueue,
    #[msg("InvalidMerkleTree")]
    InvalidMerkleTree,
    #[msg("Could not find the leaf in the queue")]
    LeafNotFound,
    #[msg("RolloverThresholdTooHigh")]
    RolloverThresholdTooHigh,
    #[msg("MerkleTreeAndQueueNotAssociated")]
    MerkleTreeAndQueueNotAssociated,
    #[msg("MerkleTreeAlreadyRolledOver")]
    MerkleTreeAlreadyRolledOver,
    #[msg("NotReadyForRollover")]
    NotReadyForRollover,
    #[msg("RolloverNotConfigured")]
    RolloverNotConfigured,
    #[msg("NotAllLeavesProcessed")]
    NotAllLeavesProcessed,
    #[msg("InvalidQueueType")]
    InvalidQueueType,
    #[msg("InputElementsEmpty")]
    InputElementsEmpty,
}
