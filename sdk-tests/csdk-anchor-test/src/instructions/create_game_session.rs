use anchor_lang::{prelude::*, solana_program::sysvar::clock::Clock};
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

pub fn create_game_session<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateGameSession<'info>>,
    session_id: u64,
    game_type: String,
    proof: ValidityProof,
    compressed_address: [u8; 32],
    address_tree_info: PackedAddressTreeInfo,
    output_state_tree_index: u8,
) -> Result<()> {
    let game_session = &mut ctx.accounts.game_session;

    // Load config from the config account
    let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;

    // Set your account data.
    game_session.session_id = session_id;
    game_session.player = ctx.accounts.player.key();
    game_session.game_type = game_type;
    game_session.start_time = Clock::get()?.unix_timestamp as u64;
    game_session.end_time = None;
    game_session.score = 0;

    // Check that rent recipient matches your config.
    if ctx.accounts.rent_sponsor.key() != config.rent_sponsor {
        return Err(ProgramError::from(ErrorCode::RentRecipientMismatch).into());
    }

    // Create CPI accounts.
    let player_account_info = ctx.accounts.player.to_account_info();
    let cpi_accounts = CpiAccounts::new(
        &player_account_info,
        ctx.remaining_accounts,
        LIGHT_CPI_SIGNER,
    );

    // Prepare new address params. The cpda takes the address of the
    // compressible pda account as seed.
    let new_address_params = address_tree_info
        .into_new_address_params_assigned_packed(game_session.key().to_bytes().into(), Some(0));

    let game_session_info = game_session.to_account_info();
    let game_session_data_mut = &mut **game_session;
    let compressed_info = prepare_compressed_account_on_init::<GameSession>(
        &game_session_info,
        game_session_data_mut,
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

    game_session.close(ctx.accounts.rent_sponsor.to_account_info())?;

    Ok(())
}
