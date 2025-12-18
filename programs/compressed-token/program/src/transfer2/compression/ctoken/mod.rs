use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_ctoken_interface::instructions::transfer2::{
    ZCompressedTokenInstructionDataTransfer2, ZCompression,
};
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;

use super::validate_compression_mode_fields;
use crate::extensions::MintExtensionChecks;

mod compress_and_close;
mod compress_or_decompress_ctokens;
mod inputs;

pub use compress_and_close::close_for_compress_and_close;
pub use compress_or_decompress_ctokens::{
    compress_or_decompress_ctokens, process_compressible_extension,
};
pub use inputs::{CTokenCompressionInputs, CompressAndCloseInputs};

/// Process compression/decompression for ctoken accounts.
#[profile]
pub(super) fn process_ctoken_compressions<'a>(
    inputs: &'a ZCompressedTokenInstructionDataTransfer2<'a>,
    compression: &ZCompression,
    token_account_info: &'a AccountInfo,
    packed_accounts: &'a ProgramPackedAccounts<'a, AccountInfo>,
    mint_checks: Option<MintExtensionChecks>,
    transfer_amount: &mut u64,
    lamports_budget: &mut u64,
) -> Result<(), anchor_lang::prelude::ProgramError> {
    // Validate compression fields for the given mode
    validate_compression_mode_fields(compression)?;

    // Create inputs struct with all required accounts extracted
    let compression_inputs = CTokenCompressionInputs::from_compression(
        compression,
        token_account_info,
        inputs,
        packed_accounts,
        mint_checks,
    )?;

    compress_or_decompress_ctokens(compression_inputs, transfer_amount, lamports_budget)
}
