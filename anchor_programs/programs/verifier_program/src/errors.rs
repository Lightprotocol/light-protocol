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
    WrongTxIntegrityHash
}
