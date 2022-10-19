use anchor_lang::prelude::*;

#[error_code]
pub enum VerifierError {
    #[msg("WrongSigner")]
    WrongSigner,

}
