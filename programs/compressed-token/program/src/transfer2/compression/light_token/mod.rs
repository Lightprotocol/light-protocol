use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_program_profiler::profile;
use light_token_interface::instructions::transfer2::{
    ZCompressedTokenInstructionDataTransfer2, ZCompression,
};
use pinocchio::account_info::AccountInfo;

use super::validate_compression_mode_fields;

mod compress_and_close;
mod compress_or_decompress_tokens;
mod inputs;

pub use compress_and_close::close_for_compress_and_close;
pub use compress_or_decompress_tokens::compress_or_decompress_tokens;
pub use inputs::{CompressAndCloseInputs, TokenCompressionInputs};

/// Process compression/decompression for light token accounts.
#[profile]
pub(super) fn process_light_token_compressions(
    inputs: &ZCompressedTokenInstructionDataTransfer2,
    compression: &ZCompression,
    token_account_info: &AccountInfo,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
    transfer_amount: &mut u64,
    lamports_budget: &mut u64,
) -> Result<(), anchor_lang::prelude::ProgramError> {
    // Validate compression fields for the given mode
    validate_compression_mode_fields(compression)?;

    // Create inputs struct with all required accounts extracted
    let compression_inputs = TokenCompressionInputs::from_compression(
        compression,
        token_account_info,
        inputs,
        packed_accounts,
    )?;

    compress_or_decompress_tokens(compression_inputs, transfer_amount, lamports_budget)
}
