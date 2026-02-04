//! Error types for light-token-pinocchio SDK.

use pinocchio::program_error::ProgramError;

/// Result type for light-token-pinocchio specific errors
pub type LightTokenResult<T> = core::result::Result<T, LightTokenError>;

/// Errors specific to high-level token operations.
/// Error codes start at 17500 to avoid conflicts with other Light Protocol errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightTokenError {
    SplInterfaceRequired = 17500,
    IncompleteSplInterface = 17501,
    UseRegularSplTransfer = 17502,
    CannotDetermineAccountType = 17503,
    MissingMintAccount = 17504,
    MissingSplTokenProgram = 17505,
    MissingSplInterfacePda = 17506,
    MissingSplInterfacePdaBump = 17507,
    SplTokenProgramMismatch = 17508,
    InvalidAccountData = 17509,
    SerializationError = 17510,
    MissingCpiContext = 17511,
    MissingCpiAuthority = 17512,
    MissingOutputQueue = 17513,
    MissingStateMerkleTree = 17514,
    MissingAddressMerkleTree = 17515,
    MissingLightSystemProgram = 17516,
    MissingRegisteredProgramPda = 17517,
    MissingAccountCompressionAuthority = 17518,
    MissingAccountCompressionProgram = 17519,
    MissingSystemProgram = 17520,
}

impl From<LightTokenError> for ProgramError {
    fn from(e: LightTokenError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl From<LightTokenError> for u32 {
    fn from(e: LightTokenError) -> Self {
        e as u32
    }
}
