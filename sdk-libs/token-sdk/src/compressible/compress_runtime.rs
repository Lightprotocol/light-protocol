//! Runtime helpers for compressing PDAs to Light Protocol.

use light_compressed_account::instruction_data::{
    data::NewAddressParamsAssignedPacked, with_account_info::CompressedAccountInfo,
};
use light_sdk::{
    cpi::{
        v2::{CpiAccounts, LightSystemProgramCpi},
        InvokeLightSystemProgram, LightCpiInstruction,
    },
    instruction::ValidityProof,
};
use light_sdk_types::CpiSigner;
use solana_program_error::ProgramError;

use crate::error::LightTokenError;

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
    proof: ValidityProof,
    new_addresses: &[NewAddressParamsAssignedPacked],
    compressed_infos: &[CompressedAccountInfo],
    cpi_accounts: &CpiAccounts<'_, 'info>,
) -> Result<(), ProgramError> {
    let cpi_context_account = cpi_accounts
        .cpi_context()
        .map_err(|_| LightTokenError::MissingCpiContext)?;
    let cpi_context_accounts = light_sdk_types::cpi_context_write::CpiContextWriteAccounts {
        fee_payer: cpi_accounts.fee_payer(),
        authority: cpi_accounts
            .authority()
            .map_err(|_| LightTokenError::MissingCpiAuthority)?,
        cpi_context: cpi_context_account,
        cpi_signer,
    };

    LightSystemProgramCpi::new_cpi(cpi_signer, proof)
        .with_new_addresses(new_addresses)
        .with_account_infos(compressed_infos)
        .write_to_cpi_context_first()
        .invoke_write_to_cpi_context_first(cpi_context_accounts)?;

    Ok(())
}
