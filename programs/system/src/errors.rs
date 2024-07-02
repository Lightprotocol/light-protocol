use anchor_lang::prelude::*;

#[error_code]
pub enum SystemProgramError {
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
    #[msg("InvalidNoopPubkey")]
    InvalidNoopPubkey,
    #[msg("InvalidAddress")]
    InvalidAddress,
    #[msg("DeriveAddressError")]
    DeriveAddressError,
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
    #[msg("WriteAccessCheckFailed")]
    WriteAccessCheckFailed,
    #[msg("InvokingProgramNotProvided")]
    InvokingProgramNotProvided,
    #[msg("InvalidCapacity")]
    InvalidCapacity,
    #[msg("InvalidMerkleTreeOwner")]
    InvalidMerkleTreeOwner,
    #[msg("ProofIsNone")]
    ProofIsNone,
    #[msg("Proof is some but no input compressed accounts or new addresses provided.")]
    ProofIsSome,
    #[msg("EmptyInputs")]
    EmptyInputs,
    #[msg("CpiContextAccountUndefined")]
    CpiContextAccountUndefined,
    #[msg("CpiContextEmpty")]
    CpiContextEmpty,
    #[msg("CpiContextMissing")]
    CpiContextMissing,
    #[msg("DecompressionRecipienDefined")]
    DecompressionRecipienDefined,
    #[msg("SolPoolPdaDefined")]
    SolPoolPdaDefined,
    #[msg("AppendStateFailed")]
    AppendStateFailed,
    #[msg("The instruction is not callable")]
    InstructionNotCallable,
    #[msg("CpiContextFeePayerMismatch")]
    CpiContextFeePayerMismatch,
    #[msg("CpiContextAssociatedMerkleTreeMismatch")]
    CpiContextAssociatedMerkleTreeMismatch,
    #[msg("NoInputs")]
    NoInputs,
    #[msg("Input merkle tree indices are not in ascending order.")]
    InputMerkleTreeIndicesNotInOrder,
    #[msg("Output merkle tree indices are not in ascending order.")]
    OutputMerkleTreeIndicesNotInOrder,
}
