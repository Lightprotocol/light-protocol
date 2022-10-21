use anchor_lang::prelude::*;

#[error_code]
pub enum VerifierSdkError {
    #[msg("Incompatible Verifying Key with number of public inputs")]
    IncompatibleVerifyingKeyWithNrPublicInputs,
    #[msg("WrongPubAmount")]
    WrongPubAmount,
    #[msg("WrongTxIntegrityHash")]
    WrongTxIntegrityHash,
    #[msg("ProofVerificationFailed")]
    ProofVerificationFailed,
    #[msg("Transaction was not executed completely")]
    TransactionIncomplete,
    #[msg("Invalid number of Nullifieraccounts")]
    InvalidNrNullifieraccounts,
    #[msg("Invalid number of Leavesaccounts")]
    InvalidNrLeavesaccounts,
    #[msg("Invalid merkle tree root")]
    InvalidMerkleTreeRoot,
    #[msg("InconsistentMintProofSenderOrRecipient")]
    InconsistentMintProofSenderOrRecipient,
    #[msg("InvalidUtxoSize")]
    InvalidUtxoSize,
    #[msg("CloseAccountFailed")]
    CloseAccountFailed
}
