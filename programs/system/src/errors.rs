use light_account_checks::error::AccountError;
use light_batched_merkle_tree::errors::BatchedMerkleTreeError;
use light_concurrent_merkle_tree::errors::ConcurrentMerkleTreeError;
use light_indexed_merkle_tree::errors::IndexedMerkleTreeError;
use light_zero_copy::errors::ZeroCopyError;
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
    #[error("Borrowing data failed")]
    BorrowingDataFailed,
    #[error("DuplicateAccountInInputsAndReadOnly")]
    DuplicateAccountInInputsAndReadOnly,
    #[error("CpiContextDeactivated")]
    CpiContextDeactivated,
    #[error("CPI context account doesn't exist, but CPI context passed as set_context or first_set_context")]
    CpiContextPassedAsSetContext,
    #[error("Invalid CPI context account owner")]
    InvalidCpiContextOwner,
    #[error("Invalid CPI context account discriminator")]
    InvalidCpiContextDiscriminator,
    #[error("Account index out of bounds")]
    InvalidAccountIndex,
    #[error("Account compression CPI data exceeds 10KB limit")]
    AccountCompressionCpiDataExceedsLimit,
    #[error("AddressOwnerIndexOutOfBounds")]
    AddressOwnerIndexOutOfBounds,
    #[error("AddressAssignedAccountIndexOutOfBounds")]
    AddressAssignedAccountIndexOutOfBounds,
    #[error("InputMerkleTreeIndexOutOfBounds (can be output queue for V2 state trees, Merkle tree for V1 state trees")]
    InputMerkleTreeIndexOutOfBounds,
    #[error("OutputMerkleTreeIndexOutOfBounds (can be output queue for V2 state trees, Merkle tree for V1 state trees")]
    OutputMerkleTreeIndexOutOfBounds,
    #[error("Packed Account index out of bounds index.")]
    PackedAccountIndexOutOfBounds,
    #[error("Unimplemented.")]
    Unimplemented,
    #[error("Missing legacy Merkle tree context")]
    MissingLegacyMerkleContext,
    #[error("Batched Merkle tree error {0}")]
    BatchedMerkleTreeError(#[from] BatchedMerkleTreeError),
    #[error("Concurrent Merkle tree error {0}")]
    ConcurrentMerkleTreeError(#[from] ConcurrentMerkleTreeError),
    #[error("Indexed Merkle tree error {0}")]
    IndexedMerkleTreeError(#[from] IndexedMerkleTreeError),
    #[error("Account checks error {0}")]
    AccountError(#[from] AccountError),
    #[error("Zero copy error {0}")]
    ZeroCopyError(#[from] ZeroCopyError),
    #[error("Program error code: {0}")]
    ProgramError(u64),
}

impl From<SystemProgramError> for u32 {
    fn from(e: SystemProgramError) -> u32 {
        match e {
            SystemProgramError::SumCheckFailed => 6000,
            SystemProgramError::SignerCheckFailed => 6001,
            SystemProgramError::CpiSignerCheckFailed => 6002,
            SystemProgramError::ComputeInputSumFailed => 6003,
            SystemProgramError::ComputeOutputSumFailed => 6004,
            SystemProgramError::ComputeRpcSumFailed => 6005,
            SystemProgramError::InvalidAddress => 6006,
            SystemProgramError::DeriveAddressError => 6007,
            SystemProgramError::CompressedSolPdaUndefinedForCompressSol => 6008,
            SystemProgramError::DecompressLamportsUndefinedForCompressSol => 6009,
            SystemProgramError::CompressedSolPdaUndefinedForDecompressSol => 6010,
            SystemProgramError::DeCompressLamportsUndefinedForDecompressSol => 6011,
            SystemProgramError::DecompressRecipientUndefinedForDecompressSol => 6012,
            SystemProgramError::WriteAccessCheckFailed => 6013,
            SystemProgramError::InvokingProgramNotProvided => 6014,
            SystemProgramError::InvalidCapacity => 6015,
            SystemProgramError::InvalidMerkleTreeOwner => 6016,
            SystemProgramError::ProofIsNone => 6017,
            SystemProgramError::ProofIsSome => 6018,
            SystemProgramError::EmptyInputs => 6019,
            SystemProgramError::CpiContextAccountUndefined => 6020,
            SystemProgramError::CpiContextEmpty => 6021,
            SystemProgramError::CpiContextMissing => 6022,
            SystemProgramError::DecompressionRecipientDefined => 6023,
            SystemProgramError::SolPoolPdaDefined => 6024,
            SystemProgramError::AppendStateFailed => 6025,
            SystemProgramError::InstructionNotCallable => 6026,
            SystemProgramError::CpiContextFeePayerMismatch => 6027,
            SystemProgramError::CpiContextAssociatedMerkleTreeMismatch => 6028,
            SystemProgramError::NoInputs => 6029,
            SystemProgramError::InputMerkleTreeIndicesNotInOrder => 6030,
            SystemProgramError::OutputMerkleTreeIndicesNotInOrder => 6031,
            SystemProgramError::OutputMerkleTreeNotUnique => 6032,
            SystemProgramError::DataFieldUndefined => 6033,
            SystemProgramError::ReadOnlyAddressAlreadyExists => 6034,
            SystemProgramError::ReadOnlyAccountDoesNotExist => 6035,
            SystemProgramError::HashChainInputsLenghtInconsistent => 6036,
            SystemProgramError::InvalidAddressTreeHeight => 6037,
            SystemProgramError::InvalidStateTreeHeight => 6038,
            SystemProgramError::InvalidArgument => 6039,
            SystemProgramError::InvalidAccount => 6040,
            SystemProgramError::AddressMerkleTreeAccountDiscriminatorMismatch => 6041,
            SystemProgramError::StateMerkleTreeAccountDiscriminatorMismatch => 6042,
            SystemProgramError::ProofVerificationFailed => 6043,
            SystemProgramError::InvalidAccountMode => 6044,
            SystemProgramError::InvalidInstructionDataDiscriminator => 6045,
            SystemProgramError::NewAddressAssignedIndexOutOfBounds => 6046,
            SystemProgramError::AddressIsNone => 6047,
            SystemProgramError::AddressDoesNotMatch => 6048,
            SystemProgramError::CpiContextAlreadySet => 6049,
            SystemProgramError::InvalidTreeHeight => 6050,
            SystemProgramError::TooManyOutputAccounts => 6051,
            SystemProgramError::BorrowingDataFailed => 6052,
            SystemProgramError::DuplicateAccountInInputsAndReadOnly => 6053,
            SystemProgramError::CpiContextPassedAsSetContext => 6054,
            SystemProgramError::InvalidCpiContextOwner => 6055,
            SystemProgramError::InvalidCpiContextDiscriminator => 6056,
            SystemProgramError::InvalidAccountIndex => 6057,
            SystemProgramError::AccountCompressionCpiDataExceedsLimit => 6058,
            SystemProgramError::AddressOwnerIndexOutOfBounds => 6059,
            SystemProgramError::AddressAssignedAccountIndexOutOfBounds => 6060,
            SystemProgramError::OutputMerkleTreeIndexOutOfBounds => 6061,
            SystemProgramError::PackedAccountIndexOutOfBounds => 6062,
            SystemProgramError::Unimplemented => 6063,
            SystemProgramError::CpiContextDeactivated => 6064,
            SystemProgramError::InputMerkleTreeIndexOutOfBounds => 6065,
            SystemProgramError::MissingLegacyMerkleContext => 6066,
            SystemProgramError::BatchedMerkleTreeError(e) => e.into(),
            SystemProgramError::IndexedMerkleTreeError(e) => e.into(),
            SystemProgramError::ConcurrentMerkleTreeError(e) => e.into(),
            SystemProgramError::AccountError(e) => e.into(),
            SystemProgramError::ProgramError(e) => u32::try_from(e).unwrap_or(0),
            SystemProgramError::ZeroCopyError(e) => e.into(),
        }
    }
}

impl From<SystemProgramError> for ProgramError {
    fn from(e: SystemProgramError) -> ProgramError {
        ProgramError::Custom(e.into())
    }
}
