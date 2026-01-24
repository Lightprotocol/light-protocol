//! Runtime helpers for compressed mint creation.
//!
//! These functions consolidate the CPI setup logic used by `#[derive(LightAccounts)]`
//! macro for mint creation, reducing macro complexity and SDK coupling.

use light_sdk::cpi::v2::CpiAccounts;
use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;

use crate::error::LightTokenError;
use crate::instruction::{CreateMintsCpi, CreateMintsParams, SystemAccountInfos};

/// Infrastructure accounts needed for mint creation CPI.
///
/// These accounts are passed from the user's Accounts struct.
pub struct CreateMintsInfraAccounts<'info> {
    /// Fee payer for the transaction.
    pub fee_payer: AccountInfo<'info>,
    /// CompressibleConfig account for the light-token program.
    pub compressible_config: AccountInfo<'info>,
    /// Rent sponsor PDA.
    pub rent_sponsor: AccountInfo<'info>,
    /// CPI authority PDA for signing.
    pub cpi_authority: AccountInfo<'info>,
}

/// Invoke CreateMintsCpi to create and decompress compressed mints.
///
/// This function handles:
/// - Extracting tree accounts from CpiAccounts
/// - Building the SystemAccountInfos
/// - Constructing and invoking CreateMintsCpi
///
/// # Arguments
/// * `mint_seed_accounts` - AccountInfos for mint signers (one per mint)
/// * `mint_accounts` - AccountInfos for mint PDAs (one per mint)
/// * `params` - CreateMintsParams with mint params and configuration
/// * `infra` - Infrastructure accounts from the Accounts struct
/// * `cpi_accounts` - CpiAccounts for accessing system accounts
#[inline(never)]
pub fn invoke_create_mints<'a, 'info>(
    mint_seed_accounts: &'a [AccountInfo<'info>],
    mint_accounts: &'a [AccountInfo<'info>],
    params: CreateMintsParams<'a>,
    infra: CreateMintsInfraAccounts<'info>,
    cpi_accounts: &CpiAccounts<'_, 'info>,
) -> Result<(), ProgramError> {
    // Extract tree accounts from CpiAccounts
    let output_queue = cpi_accounts
        .get_tree_account_info(params.output_queue_index as usize)
        .map_err(|_| LightTokenError::MissingOutputQueue)?
        .clone();
    let state_merkle_tree = cpi_accounts
        .get_tree_account_info(params.state_tree_index as usize)
        .map_err(|_| LightTokenError::MissingStateMerkleTree)?
        .clone();
    let address_tree = cpi_accounts
        .get_tree_account_info(params.address_tree_index as usize)
        .map_err(|_| LightTokenError::MissingAddressMerkleTree)?
        .clone();

    // Build system accounts from CpiAccounts
    let system_accounts = SystemAccountInfos {
        light_system_program: cpi_accounts
            .light_system_program()
            .map_err(|_| LightTokenError::MissingLightSystemProgram)?
            .clone(),
        cpi_authority_pda: infra.cpi_authority,
        registered_program_pda: cpi_accounts
            .registered_program_pda()
            .map_err(|_| LightTokenError::MissingRegisteredProgramPda)?
            .clone(),
        account_compression_authority: cpi_accounts
            .account_compression_authority()
            .map_err(|_| LightTokenError::MissingAccountCompressionAuthority)?
            .clone(),
        account_compression_program: cpi_accounts
            .account_compression_program()
            .map_err(|_| LightTokenError::MissingAccountCompressionProgram)?
            .clone(),
        system_program: cpi_accounts
            .system_program()
            .map_err(|_| LightTokenError::MissingSystemProgram)?
            .clone(),
    };

    // Build and invoke CreateMintsCpi
    CreateMintsCpi {
        mint_seed_accounts,
        payer: infra.fee_payer,
        address_tree,
        output_queue,
        state_merkle_tree,
        compressible_config: infra.compressible_config,
        mints: mint_accounts,
        rent_sponsor: infra.rent_sponsor,
        system_accounts,
        cpi_context_account: cpi_accounts
            .cpi_context()
            .map_err(|_| LightTokenError::MissingCpiContext)?
            .clone(),
        params,
    }
    .invoke()
}
