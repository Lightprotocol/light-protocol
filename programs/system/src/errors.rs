use anchor_lang::prelude::*;

#[error_code]
pub enum CompressedPdaError {
    #[msg("Sum check failed")]
    SumCheckFailed,
    #[msg("Signer check failed")]
    SignerCheckFailed,
    #[msg("Cpi signer check failed")]
    CpiSignerCheckFailed,
    #[msg("Computing input sum failed.")]
    ComputeInputSumFailed,
    #[msg("Computing output sum failed.")]
    ComputeOutputSumFailed,
    #[msg("Computing rpc sum failed.")]
    ComputeRpcSumFailed,
    #[msg("InUtxosAlreadyAdded")]
    InUtxosAlreadyAdded,
    #[msg("NumberOfLeavesMismatch")]
    NumberOfLeavesMismatch,
    #[msg("MerkleTreePubkeysMismatch")]
    MerkleTreePubkeysMismatch,
    #[msg("NullifierArrayPubkeysMismatch")]
    NullifierArrayPubkeysMismatch,
    #[msg("InvalidNoopPubkey")]
    InvalidNoopPubkey,
    #[msg("ProofVerificationFailed")]
    ProofVerificationFailed,
    #[msg("CompressedAccountHashError")]
    CompressedAccountHashError,
    #[msg("InvalidAddress")]
    InvalidAddress,
    #[msg("InvalidAddressQueue")]
    InvalidAddressQueue,
    #[msg("InvalidQueue")]
    InvalidQueue,
    #[msg("DeriveAddressError")]
    DeriveAddressError,
    #[msg("CompressSolTransferFailed")]
    CompressSolTransferFailed,
    #[msg("CompressedSolPdaUndefinedForCompressSol")]
    CompressedSolPdaUndefinedForCompressSol,
    #[msg("DeCompressLamportsUndefinedForCompressSol")]
    DeCompressLamportsUndefinedForCompressSol,
    #[msg("CompressedSolPdaUndefinedForDecompressSol")]
    CompressedSolPdaUndefinedForDecompressSol,
    #[msg("DeCompressLamportsUndefinedForDecompressSol")]
    DeCompressLamportsUndefinedForDecompressSol,
    #[msg("DecompressRecipientUndefinedForDecompressSol")]
    DecompressRecipientUndefinedForDecompressSol,
    #[msg("LengthMismatch")]
    LengthMismatch,
    #[msg("CpiContextAccountUndefined")]
    CpiContextAccountUndefined,
    #[msg("WriteAccessCheckFailed")]
    WriteAccessCheckFailed,
    #[msg("InvokingProgramNotProvided")]
    InvokingProgramNotProvided,
    #[msg("SignerSeedsNotProvided")]
    SignerSeedsNotProvided,
    #[msg("AdditionOverflowForDecompressSol")]
    AdditionOverflowForDecompressSol,
    #[msg("InsufficientLamportsForDecompressSol")]
    InsufficientLamportsForDecompressSol,
    #[msg("InsufficientLamportsForCompressSol")]
    CpiContextMissing,
    #[msg("InvalidMerkleTreeOwner")]
    InvalidMerkleTreeOwner,
    #[msg("ProofIsNone")]
    ProofIsNone,
    #[msg("InvalidMerkleTreeIndex")]
    InvalidMerkleTreeIndex,
    #[msg("ProofIsSome")]
    ProofIsSome,
    #[msg("EmptyInputs")]
    EmptyInputs,
    #[msg("CpiContextMismatch")]
    CpiContextProofMismatch,
    #[msg("CpiContextEmpty")]
    CpiContextEmpty,
    #[msg("HashedPubkeysCapacityMismatch")]
    HashedPubkeysCapacityMismatch,
}
