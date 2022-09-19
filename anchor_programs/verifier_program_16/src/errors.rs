use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Incompatible Verifying Key")]
    IncompatibleVerifyingKey,
    #[msg("WrongPubAmount")]
    WrongPubAmount,
    #[msg("PrepareInputsDidNotFinish")]
    PrepareInputsDidNotFinish,
    #[msg("NotLastTransactionState")]
    NotLastTransactionState,
    #[msg("Tx is not a deposit")]
    NotDeposit,
    #[msg("WrongTxIntegrityHash")]
    WrongTxIntegrityHash,
    #[msg("Closing escrow state failed relayer not timed out.")]
    NotTimedOut,
    #[msg("WrongSigner")]
    WrongSigner,
    #[msg("VerifierStateAlreadyInitialized")]
    VerifierStateAlreadyInitialized,
    #[msg("Nullifier already exists")]
    NullifierAlreadyExists,
    #[msg("Token escrow account is incorrect.")]
    IncorrectTokenEscrowAcc,
    #[msg("WrongUserTokenPda")]
    WrongUserTokenPda,
    #[msg("ProofVerificationFailed")]
    ProofVerificationFailed,
    #[msg("Transaction was not executed completely")]
    TransactionIncomplete
}
