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

pub fn create_record<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateRecord<'info>>,
    name: String,
    proof: ValidityProof,
    compressed_address: [u8; 32],
    address_tree_info: PackedAddressTreeInfo,
    output_state_tree_index: u8,
) -> Result<()> {
    let user_record = &mut ctx.accounts.user_record;

    // 1. Load config from the config account
    let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;

    user_record.owner = ctx.accounts.user.key();
    user_record.name = name;
    user_record.score = 11;

    // 2. Verify rent recipient matches config
    if ctx.accounts.rent_sponsor.key() != config.rent_sponsor {
        return Err(ProgramError::from(ErrorCode::RentRecipientMismatch).into());
    }

    // 3. Create CPI accounts
    let user_account_info = ctx.accounts.user.to_account_info();
    let cpi_accounts =
        CpiAccounts::new(&user_account_info, ctx.remaining_accounts, LIGHT_CPI_SIGNER);

    let new_address_params = address_tree_info
        .into_new_address_params_assigned_packed(user_record.key().to_bytes().into(), Some(0));

    let user_record_info = user_record.to_account_info();
    let user_record_data_mut = &mut **user_record;
    let compressed_info = prepare_compressed_account_on_init::<UserRecord>(
        &user_record_info,
        user_record_data_mut,
        compressed_address,
        new_address_params,
        output_state_tree_index,
        &cpi_accounts,
        &config.address_space,
        true, // with_data
    )?;

    LightSystemProgramCpi::new_cpi(cpi_accounts.config().cpi_signer, proof)
        .with_new_addresses(&[new_address_params])
        .with_account_infos(&[compressed_info])
        .invoke(cpi_accounts)?;

    // Close the PDA
    user_record.close(ctx.accounts.rent_sponsor.to_account_info())?;

    Ok(())
}
