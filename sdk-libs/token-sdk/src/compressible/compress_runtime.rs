//! Runtime helpers for compressing PDAs to Light Protocol.

use light_compressed_account::instruction_data::{
    cpi_context::CompressedCpiContext,
    data::NewAddressParamsAssignedPacked,
    with_account_info::{CompressedAccountInfo, InstructionDataInvokeCpiWithAccountInfo},
};
use light_sdk::cpi::{v2::CpiAccounts, InvokeLightSystemProgram};
use light_sdk_types::CpiSigner;
use solana_program_error::ProgramError;

use crate::error::LightTokenError;
// TODO: rename file
/// Write PDAs to CPI context for chaining with mint operations.
///
/// Use this when PDAs need to be written to CPI context first, which will be
/// consumed by subsequent mint operations (e.g., CreateMintsCpi).
///
/// # Arguments
/// * `cpi_signer` - CPI signer for the invoking program
/// * `proof` - Validity proof for the compression operation
/// * `new_addresses` - New address parameters for each PDA
/// * `compressed_infos` - Compressed account info for each PDA
/// * `cpi_accounts` - CPI accounts with CPI context enabled
pub fn invoke_write_pdas_to_cpi_context<'info>(
    cpi_signer: CpiSigner,
    proof: light_sdk::instruction::ValidityProof,
    new_addresses: &[NewAddressParamsAssignedPacked],
    compressed_infos: &[CompressedAccountInfo],
    cpi_accounts: &CpiAccounts<'_, 'info>,
) -> Result<(), ProgramError> {
    let cpi_context_accounts = light_sdk_types::cpi_context_write::CpiContextWriteAccounts {
        fee_payer: cpi_accounts.fee_payer(),
        authority: cpi_accounts
            .authority()
            .map_err(|_| LightTokenError::MissingCpiAuthority)?,
        cpi_context: cpi_accounts
            .cpi_context()
            .map_err(|_| LightTokenError::MissingCpiContext)?,
        cpi_signer,
    };

    let instruction_data = InstructionDataInvokeCpiWithAccountInfo {
        mode: 1,
        bump: cpi_signer.bump,
        invoking_program_id: cpi_signer.program_id.into(),
        compress_or_decompress_lamports: 0,
        is_compress: false,
        with_cpi_context: true,
        with_transaction_hash: false,
        cpi_context: CompressedCpiContext::first(),
        proof: proof.0,
        new_address_params: new_addresses.to_vec(),
        account_infos: compressed_infos.to_vec(),
        read_only_addresses: vec![],
        read_only_accounts: vec![],
    };

    instruction_data
        .invoke_write_to_cpi_context_first(cpi_context_accounts)
        .map_err(|e| ProgramError::Custom(u32::from(e)))?;

    Ok(())
}
