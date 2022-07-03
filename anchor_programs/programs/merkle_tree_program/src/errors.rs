use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Merkle tree tmp account init failed wrong pda.")]
    MtTmpPdaInitFailed,
    #[msg("Merkle tree tmp account init failed.")]
    MerkleTreeInitFailed,
    #[msg("Contract is still locked.")]
    ContractStillLocked,
    #[msg("InvalidMerkleTree.")]
    InvalidMerkleTree,
    #[msg("InvalidMerkleTreeOwner.")]
    InvalidMerkleTreeOwner,
    #[msg("PubkeyCheckFailed")]
    PubkeyCheckFailed,
    #[msg("CloseAccountFailed")]
    CloseAccountFailed,
    #[msg("WithdrawalFailed")]
    WithdrawalFailed,
    #[msg("MerkleTreeUpdateNotInRootInsert")]
    MerkleTreeUpdateNotInRootInsert,
    #[msg("InvalidNumberOfLeaves")]
    InvalidNumberOfLeaves,
    #[msg("LeafAlreadyInserted")]
    LeafAlreadyInserted,
    #[msg("WrongLeavesLastTx")]
    WrongLeavesLastTx,
    #[msg("FirstLeavesPdaIncorrectIndex")]
    FirstLeavesPdaIncorrectIndex,
    #[msg("NullifierAlreadyExists")]
    NullifierAlreadyExists
}
