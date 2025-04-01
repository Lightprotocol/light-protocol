use light_verifier::VerifierError;
// use anchor_lang::error_code;
use pinocchio::program_error::ProgramError;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum SystemProgramError {
    #[error("Sum check failed")]
    SumCheckFailed,
    #[error("Signer check failed")]
    SignerCheckFailed,
    #[error("Cpi signer check failed")]
    CpiSignerCheckFailed,
    #[error("Computing input sum failed.")]
    ComputeInputSumFailed,
    #[error("Computing output sum failed.")]
    ComputeOutputSumFailed,
    #[error("Computing rpc sum failed.")]
    ComputeRpcSumFailed,
    #[error("InvalidAddress")]
    InvalidAddress,
    #[error("DeriveAddressError")]
    DeriveAddressError,
    #[error("CompressedSolPdaUndefinedForCompressSol")]
    CompressedSolPdaUndefinedForCompressSol,
    #[error("DeCompressLamportsUndefinedForCompressSol")]
    DeCompressLamportsUndefinedForCompressSol,
    #[error("CompressedSolPdaUndefinedForDecompressSol")]
    CompressedSolPdaUndefinedForDecompressSol,
    #[error("DeCompressLamportsUndefinedForDecompressSol")]
    DeCompressLamportsUndefinedForDecompressSol,
    #[error("DecompressRecipientUndefinedForDecompressSol")]
    DecompressRecipientUndefinedForDecompressSol,
    #[error("WriteAccessCheckFailed")]
    WriteAccessCheckFailed,
    #[error("InvokingProgramNotProvided")]
    InvokingProgramNotProvided,
    #[error("InvalidCapacity")]
    InvalidCapacity,
    #[error("InvalidMerkleTreeOwner")]
    InvalidMerkleTreeOwner,
    #[error("ProofIsNone")]
    ProofIsNone,
    #[error("Proof is some but no input compressed accounts or new addresses provided.")]
    ProofIsSome,
    #[error("EmptyInputs")]
    EmptyInputs,
    #[error("CpiContextAccountUndefined")]
    CpiContextAccountUndefined,
    #[error("CpiContextEmpty")]
    CpiContextEmpty,
    #[error("CpiContextMissing")]
    CpiContextMissing,
    #[error("DecompressionRecipientDefined")]
    DecompressionRecipientDefined,
    #[error("SolPoolPdaDefined")]
    SolPoolPdaDefined,
    #[error("AppendStateFailed")]
    AppendStateFailed,
    #[error("The instruction is not callable")]
    InstructionNotCallable,
    #[error("CpiContextFeePayerMismatch")]
    CpiContextFeePayerMismatch,
    #[error("CpiContextAssociatedMerkleTreeMismatch")]
    CpiContextAssociatedMerkleTreeMismatch,
    #[error("NoInputs")]
    NoInputs,
    #[error("Input merkle tree indices are not in ascending order.")]
    InputMerkleTreeIndicesNotInOrder,
    #[error("Output merkle tree indices are not in ascending order.")]
    OutputMerkleTreeIndicesNotInOrder,
    #[error("OutputMerkleTreeNotUnique")]
    OutputMerkleTreeNotUnique,
    #[error("DataFieldUndefined")]
    DataFieldUndefined,
    #[error("ReadOnlyAddressAlreadyExists")]
    ReadOnlyAddressAlreadyExists,
    #[error("ReadOnlyAccountDoesNotExist")]
    ReadOnlyAccountDoesNotExist,
    #[error("HashChainInputsLenghtInconsistent")]
    HashChainInputsLenghtInconsistent,
    #[error("InvalidAddressTreeHeight")]
    InvalidAddressTreeHeight,
    #[error("InvalidStateTreeHeight")]
    InvalidStateTreeHeight,
    #[error("InvalidArgument")]
    InvalidArgument,
    #[error("InvalidAccount")]
    InvalidAccount,
    #[error("AddressMerkleTreeAccountDiscriminatorMismatch")]
    AddressMerkleTreeAccountDiscriminatorMismatch,
    #[error("StateMerkleTreeAccountDiscriminatorMismatch")]
    StateMerkleTreeAccountDiscriminatorMismatch,
    #[error("Verifier Error")]
    VerifierError,
}

impl From<SystemProgramError> for ProgramError {
    fn from(e: SystemProgramError) -> ProgramError {
        ProgramError::Custom(e as u32 + 6000)
    }
}
