use anchor_lang::prelude::*;
use light_sdk::{
    account::LightAccount,
    cpi::{
        v1::CpiAccounts,
        v2::{lowlevel::InstructionDataInvokeCpiWithReadOnly, LightSystemProgramCpi},
        InvokeLightSystemProgram, LightCpiInstruction,
    },
    instruction::{account_meta::CompressedAccountMetaBurn, ValidityProof},
};

use crate::{MyCompressedAccount, UpdateNestedData, LIGHT_CPI_SIGNER};

#[error_code]
pub enum ReadOnlyError {
    #[msg("Invalid account")]
    InvalidAccount,
}

/// Test read-only account validation with SHA256 hasher using LightSystemProgramCpi (v2)
pub fn process_read_sha256_light_system_cpi<'info>(
    ctx: Context<'_, '_, '_, 'info, UpdateNestedData<'info>>,
    proof: ValidityProof,
    my_compressed_account: MyCompressedAccount,
    account_meta: CompressedAccountMetaBurn,
) -> Result<()> {
    let light_cpi_accounts = CpiAccounts::new(
        ctx.accounts.signer.as_ref(),
        ctx.remaining_accounts,
        crate::LIGHT_CPI_SIGNER,
    );

    // Create read-only account with SHA256 hasher
    let tree_pubkeys = light_cpi_accounts
        .tree_pubkeys()
        .map_err(|_| error!(ReadOnlyError::InvalidAccount))?;

    let read_only_account = LightAccount::<MyCompressedAccount>::new_read_only(
        &crate::ID,
        &account_meta,
        my_compressed_account,
        tree_pubkeys.as_slice(),
    )?;

    LightSystemProgramCpi::new_cpi(LIGHT_CPI_SIGNER, proof)
        .mode_v1()
        .with_light_account(read_only_account)?
        .invoke(light_cpi_accounts)?;

    Ok(())
}

/// Test read-only account validation with Poseidon hasher using LightSystemProgramCpi (v2)
pub fn process_read_poseidon_light_system_cpi<'info>(
    ctx: Context<'_, '_, '_, 'info, UpdateNestedData<'info>>,
    proof: ValidityProof,
    my_compressed_account: MyCompressedAccount,
    account_meta: CompressedAccountMetaBurn,
) -> Result<()> {
    let light_cpi_accounts = CpiAccounts::new(
        ctx.accounts.signer.as_ref(),
        ctx.remaining_accounts,
        crate::LIGHT_CPI_SIGNER,
    );

    // Create read-only account with Poseidon hasher
    let tree_pubkeys = light_cpi_accounts
        .tree_pubkeys()
        .map_err(|_| error!(ReadOnlyError::InvalidAccount))?;

    let read_only_account =
        light_sdk::account::poseidon::LightAccount::<MyCompressedAccount>::new_read_only(
            &crate::ID,
            &account_meta,
            my_compressed_account,
            tree_pubkeys.as_slice(),
        )?;

    LightSystemProgramCpi::new_cpi(LIGHT_CPI_SIGNER, proof)
        .mode_v1()
        .with_light_account_poseidon(read_only_account)?
        .invoke(light_cpi_accounts)?;

    Ok(())
}

/// Test read-only account with SHA256 hasher using InstructionDataInvokeCpiWithReadOnly (v2)
pub fn process_read_sha256_lowlevel<'info>(
    ctx: Context<'_, '_, '_, 'info, UpdateNestedData<'info>>,
    proof: ValidityProof,
    my_compressed_account: MyCompressedAccount,
    account_meta: CompressedAccountMetaBurn,
) -> Result<()> {
    let light_cpi_accounts = CpiAccounts::new(
        ctx.accounts.signer.as_ref(),
        ctx.remaining_accounts,
        crate::LIGHT_CPI_SIGNER,
    );

    // Create read-only account with SHA256 hasher
    let tree_pubkeys = light_cpi_accounts
        .tree_pubkeys()
        .map_err(|_| error!(ReadOnlyError::InvalidAccount))?;

    let read_only_account = LightAccount::<MyCompressedAccount>::new_read_only(
        &crate::ID,
        &account_meta,
        my_compressed_account,
        tree_pubkeys.as_slice(),
    )?;

    InstructionDataInvokeCpiWithReadOnly::new_cpi(LIGHT_CPI_SIGNER, proof)
        .mode_v1()
        .with_light_account(read_only_account)?
        .invoke(light_cpi_accounts)?;

    Ok(())
}

/// Test read-only account with Poseidon hasher using InstructionDataInvokeCpiWithReadOnly (v2)
pub fn process_read_poseidon_lowlevel<'info>(
    ctx: Context<'_, '_, '_, 'info, UpdateNestedData<'info>>,
    proof: ValidityProof,
    my_compressed_account: MyCompressedAccount,
    account_meta: CompressedAccountMetaBurn,
) -> Result<()> {
    let light_cpi_accounts = CpiAccounts::new(
        ctx.accounts.signer.as_ref(),
        ctx.remaining_accounts,
        crate::LIGHT_CPI_SIGNER,
    );

    // Create read-only account with Poseidon hasher
    let tree_pubkeys = light_cpi_accounts
        .tree_pubkeys()
        .map_err(|_| error!(ReadOnlyError::InvalidAccount))?;

    let read_only_account =
        light_sdk::account::poseidon::LightAccount::<MyCompressedAccount>::new_read_only(
            &crate::ID,
            &account_meta,
            my_compressed_account,
            tree_pubkeys.as_slice(),
        )?;

    InstructionDataInvokeCpiWithReadOnly::new_cpi(LIGHT_CPI_SIGNER, proof)
        .mode_v1()
        .with_light_account_poseidon(read_only_account)?
        .invoke(light_cpi_accounts)?;

    Ok(())
}
