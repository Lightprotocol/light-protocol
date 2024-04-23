use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

pub mod append_state;
pub mod event;
pub mod instructions;
pub mod utils;
pub use instructions::*;
pub use sol_compression::*;
pub mod compressed_account;
pub mod create_address;
pub mod nullify_state;
pub mod sdk;
pub mod sol_compression;
pub mod verify_state;
pub mod verifying_keys;

declare_id!("6UqiSPd2mRCTTwkzhcs1M6DGYsqHWd5jiPueX3LwDMXQ");

#[error_code]
pub enum ErrorCode {
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
    #[msg("InvalidPublicInputsLength")]
    InvalidPublicInputsLength,
    #[msg("Decompress G1 Failed")]
    DecompressG1Failed,
    #[msg("Decompress G2 Failed")]
    DecompressG2Failed,
    #[msg("CreateGroth16VerifierFailed")]
    CreateGroth16VerifierFailed,
    #[msg("ProofVerificationFailed")]
    ProofVerificationFailed,
    #[msg("PublicInputsTryIntoFailed")]
    PublicInputsTryIntoFailed,
    #[msg("CompressedAccountHashError")]
    CompressedAccountHashError,
    #[msg("InvalidAddress")]
    InvalidAddress,
    #[msg("InvalidAddressQueue")]
    InvalidAddressQueue,
    #[msg("InvalidNullifierQueue")]
    InvalidNullifierQueue,
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
    #[msg("DelegateUndefined while delegated amount is defined")]
    DelegateUndefined,
}

// // TODO(vadorovsky): Come up with some less glass chewy way of reusing
// // our light-heap allocator if it's already used in some dependency.
// #[cfg(all(feature = "custom-heap", target_os = "solana"))]
// pub use account_compression::GLOBAL_ALLOCATOR;

#[program]
pub mod light_compressed_pda {

    use self::instructions::{
        process_execute_compressed_transaction,
        InstructionDataTransfer,
        //  into_inputs,InstructionDataTransfer2,
    };
    use super::*;

    /// Initializes the compressed sol pda.
    /// This pda is used to store compressed sol for the protocol.
    pub fn init_compress_sol_pda(_ctx: Context<InitializeCompressedSolPda>) -> Result<()> {
        msg!("initialized compress sol pda");
        Ok(())
    }

    /// This function can be used to transfer sol and execute any other compressed transaction.
    /// Instruction data is not optimized for space.
    /// This method can be called by cpi so that instruction data can be compressed with a custom algorithm.
    pub fn execute_compressed_transaction<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        msg!("execute_compressed_transaction");
        let inputs: InstructionDataTransfer =
            InstructionDataTransfer::deserialize(&mut inputs.as_slice())?;
        inputs.check_input_lengths()?;
        process_execute_compressed_transaction(&inputs, &ctx)
    }

    // /// This function can be used to transfer sol and execute any other compressed transaction.
    // /// Instruction data is optimized for space.
    // pub fn execute_compressed_transaction2<'a, 'b, 'c: 'info, 'info>(
    //     ctx: Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    //     inputs: Vec<u8>,
    // ) -> Result<crate::event::PublicTransactionEvent> {
    //     let inputs: InstructionDataTransfer2 = InstructionDataTransfer2::try_deserialize_unchecked(
    //         &mut [vec![0u8; 8], inputs].concat().as_slice(),
    //     )?;
    //     let inputs = into_inputs(
    //         inputs,
    //         &ctx.accounts
    //             .to_account_infos()
    //             .iter()
    //             .map(|a| a.key())
    //             .collect::<Vec<Pubkey>>(),
    //         &ctx.remaining_accounts
    //             .iter()
    //             .map(|a| a.key())
    //             .collect::<Vec<Pubkey>>(),
    //     )?;
    //     process_execute_compressed_transaction(&inputs, &ctx)
    // }

    // TODO: add compress and decompress sol as a wrapper around process_execute_compressed_transaction

    // TODO: add create_pda as a wrapper around process_execute_compressed_transaction
}
