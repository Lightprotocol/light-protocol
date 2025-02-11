use anchor_lang::prelude::*;

#[error_code]
pub enum AccountCompressionErrorCode {
    AddressMerkleTreeAccountDiscriminatorMismatch,
    EmptyLeaf,
    InputDeserializationFailed,
    #[msg("InputElementsEmpty")]
    InputElementsEmpty,
    #[msg("InsufficientRolloverFee")]
    InsufficientRolloverFee,
    #[msg("Integer overflow")]
    IntegerOverflow,
    InvalidAccount,
    #[msg("InvalidAccountBalance")]
    InvalidAccountBalance,
    #[msg("InvalidAccountSize")]
    InvalidAccountSize,
    #[msg("InvalidAuthority")]
    InvalidAuthority,
    InvalidGroup,
    #[msg("InvalidMerkleProof")]
    InvalidMerkleProof,
    #[msg("Provided noop program public key is invalid")]
    InvalidNoopPubkey,
    #[msg("InvalidQueueType")]
    InvalidQueueType,
    #[msg("Invalid sequence threshold")]
    InvalidSequenceThreshold,
    #[msg("Could not find the leaf in the queue")]
    LeafNotFound,
    #[msg("MerkleTreeAlreadyRolledOver")]
    MerkleTreeAlreadyRolledOver,
    #[msg("MerkleTreeAndQueueNotAssociated")]
    MerkleTreeAndQueueNotAssociated,
    #[msg("NoLeavesForMerkleTree")]
    NoLeavesForMerkleTree,
    #[msg("NotAllLeavesProcessed")]
    NotAllLeavesProcessed,
    #[msg("NotReadyForRollover")]
    NotReadyForRollover,
    #[msg("Number of change log indices mismatch")]
    NumberOfChangeLogIndicesMismatch,
    #[msg("Number of indices mismatch")]
    NumberOfIndicesMismatch,
    #[msg(
        "Leaves <> remaining accounts mismatch. The number of remaining accounts must match the number of leaves."
    )]
    NumberOfLeavesMismatch,
    #[msg("NumberOfProofsMismatch")]
    NumberOfProofsMismatch,
    ProofLengthMismatch,
    RegistryProgramIsNone,
    #[msg("RolloverNotConfigured")]
    RolloverNotConfigured,
    StateMerkleTreeAccountDiscriminatorMismatch,
    #[msg("The maximum number of leaves is 255")]
    TooManyLeaves,
    TxHashUndefined,
    UnsupportedAdditionalBytes,
    #[msg("Unsupported canopy depth")]
    UnsupportedCanopyDepth,
    #[msg("Unsupported close threshold")]
    UnsupportedCloseThreshold,
    #[msg("Unsupported Merkle tree height")]
    UnsupportedHeight,
    UnsupportedParameters,
    V1AccountMarkedAsProofByIndex,
}
