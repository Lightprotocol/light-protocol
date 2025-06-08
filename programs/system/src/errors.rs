// use anchor_lang::error_code;
use pinocchio::program_error::ProgramError;
use solana_msg::msg;
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
    #[error("DecompressLamportsUndefinedForCompressSol")]
    DecompressLamportsUndefinedForCompressSol,
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
    #[error("Proof verification failed.")]
    ProofVerificationFailed,
    #[error("Invalid account mode.")]
    InvalidAccountMode,
    #[error("InvalidInstructionDataDiscriminator")]
    InvalidInstructionDataDiscriminator,
    #[error("NewAddressAssignedIndexOutOfBounds")]
    NewAddressAssignedIndexOutOfBounds,
    #[error("AddressIsNone")]
    AddressIsNone,
    #[error("AddressDoesNotMatch")]
    AddressDoesNotMatch,
    #[error("CpiContextAlreadySet")]
    CpiContextAlreadySet,
    #[error("InvalidTreeHeight")]
    InvalidTreeHeight,
    #[error("TooManyOutputAccounts")]
    TooManyOutputAccounts,
    #[error("CompressedAccountError")]
    CompressedAccountError,
    #[error("HasherError")]
    HasherError,
}

impl From<SystemProgramError> for ProgramError {
    fn from(e: SystemProgramError) -> ProgramError {
        ProgramError::Custom(e as u32 + 6000)
    }
}

impl From<light_compressed_account::CompressedAccountError> for SystemProgramError {
    fn from(err: light_compressed_account::CompressedAccountError) -> Self {
        msg!("Compressed account error {}", err);
        SystemProgramError::CompressedAccountError
    }
}

impl From<light_hasher::HasherError> for SystemProgramError {
    fn from(err: light_hasher::HasherError) -> Self {
        msg!("Hasher error {}", err);
        SystemProgramError::HasherError
    }
}
