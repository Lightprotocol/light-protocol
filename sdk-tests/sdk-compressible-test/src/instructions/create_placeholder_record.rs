use anchor_lang::prelude::*;
use light_sdk::{
    compressible::{
        compress_account_on_init::prepare_compressed_account_on_init, CompressibleConfig,
    },
    cpi::{
        v2::{CpiAccounts, LightSystemProgramCpi},
        InvokeLightSystemProgram, LightCpiInstruction,
    },
    instruction::{PackedAddressTreeInfo, ValidityProof},
};

use crate::{errors::ErrorCode, instruction_accounts::*, state::*, LIGHT_CPI_SIGNER};

pub fn create_placeholder_record<'info>(
    ctx: Context<'_, '_, '_, 'info, CreatePlaceholderRecord<'info>>,
    placeholder_id: u64,
    name: String,
    proof: ValidityProof,
    compressed_address: [u8; 32],
    address_tree_info: PackedAddressTreeInfo,
    output_state_tree_index: u8,
) -> Result<()> {
    let placeholder_record = &mut ctx.accounts.placeholder_record;

    let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;

    placeholder_record.owner = ctx.accounts.user.key();
    placeholder_record.name = name;
    placeholder_record.placeholder_id = placeholder_id;

    // Verify rent recipient matches config
    if ctx.accounts.rent_sponsor.key() != config.rent_sponsor {
        return Err(ProgramError::from(ErrorCode::RentRecipientMismatch).into());
    }

    // Create CPI accounts
    let user_account_info = ctx.accounts.user.to_account_info();
    let cpi_accounts =
        CpiAccounts::new(&user_account_info, ctx.remaining_accounts, LIGHT_CPI_SIGNER);

    let new_address_params = address_tree_info.into_new_address_params_assigned_packed(
        placeholder_record.key().to_bytes().into(),
        Some(0),
    );

    let placeholder_info = placeholder_record.to_account_info();
    let placeholder_data_mut = &mut **placeholder_record;
    let compressed_info = prepare_compressed_account_on_init::<PlaceholderRecord>(
        &placeholder_info,
        placeholder_data_mut,
        &config,
        compressed_address,
        new_address_params,
        output_state_tree_index,
        &cpi_accounts,
        &config.address_space,
        false, // with_data = false for empty compressed account
    )?;

    LightSystemProgramCpi::new_cpi(cpi_accounts.config().cpi_signer, proof)
        .with_new_addresses(&[new_address_params])
        .with_account_infos(&[compressed_info])
        .invoke(cpi_accounts)?;

    // Note: PDA is NOT closed in this example (compression_info is set, account remains)
    Ok(())
}
