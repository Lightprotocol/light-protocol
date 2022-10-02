use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum Groth16Error {
    #[error("Incompatible Verifying Key with number of public inputs")]
    IncompatibleVerifyingKeyWithNrPublicInputs,
    #[error("ProofVerificationFailed")]
    ProofVerificationFailed,
    #[error("InvalidG1Length")]
    InvalidG1Length,
    #[error("InvalidG2Length")]
    InvalidG2Length,
    #[error("InvalidPublicInputsLength")]
    InvalidPublicInputsLength
}
