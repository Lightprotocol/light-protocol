use anchor_lang::prelude::*;

#[error_code]
pub enum AccountCompressionErrorCode {
    #[msg("Integer overflow")]
    IntegerOverflow,
    #[msg("InvalidAuthority")]
    InvalidAuthority,
    #[msg(
        "Leaves <> remaining accounts mismatch. The number of remaining accounts must match the number of leaves."
    )]
    NumberOfLeavesMismatch,
    #[msg("Provided noop program public key is invalid")]
    InvalidNoopPubkey,
    #[msg("Number of change log indices mismatch")]
    NumberOfChangeLogIndicesMismatch,
    #[msg("Number of indices mismatch")]
    NumberOfIndicesMismatch,
    #[msg("NumberOfProofsMismatch")]
    NumberOfProofsMismatch,
    #[msg("InvalidMerkleProof")]
    InvalidMerkleProof,
    #[msg("Could not find the leaf in the queue")]
    LeafNotFound,
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
    #[msg("NoLeavesForMerkleTree")]
    NoLeavesForMerkleTree,
    #[msg("InvalidAccountSize")]
    InvalidAccountSize,
    #[msg("InsufficientRolloverFee")]
    InsufficientRolloverFee,
    #[msg("Unsupported Merkle tree height")]
    UnsupportedHeight,
    #[msg("Unsupported canopy depth")]
    UnsupportedCanopyDepth,
    #[msg("Invalid sequence threshold")]
    InvalidSequenceThreshold,
    #[msg("Unsupported close threshold")]
    UnsupportedCloseThreshold,
    #[msg("InvalidAccountBalance")]
    InvalidAccountBalance,
}
