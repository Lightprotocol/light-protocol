use anchor_lang::prelude::*;
use light_sdk::{
    compressible::CompressibleConfig,
    cpi::CpiAccounts,
    instruction::{PackedAddressTreeInfo, ValidityProof},
};

use crate::state::ZeroCopyObservation;

#[derive(Accounts)]
#[instruction(observation_id: u64)]
pub struct CreateZeroCopyObservation<'info> {
    #[account(mut)]
    pub observer: Signer<'info>,
    #[account(
        init,
        payer = observer,
        space = 8 + ZeroCopyObservation::INIT_SPACE,
        seeds = [b"zero_copy_observation", observation_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub zero_copy_observation: Account<'info, ZeroCopyObservation>,
    /// The global config account
    /// CHECK: Config is validated by the SDK's load_checked method
    pub config: AccountInfo<'info>,
    /// Rent recipient - must match config
    /// CHECK: Rent recipient is validated against the config
    #[account(mut)]
    pub rent_recipient: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

pub fn create_zero_copy_observation<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateZeroCopyObservation<'info>>,
    observation_id: u64,
    timestamp: i64,
    value: i64,
    confidence: u8,
    proof: ValidityProof,
    compressed_address: [u8; 32],
    address_tree_info: PackedAddressTreeInfo,
    output_state_tree_index: u8,
) -> Result<()> {
    let zero_copy_observation = &mut ctx.accounts.zero_copy_observation;

    // Initialize the observation data
    zero_copy_observation.observer = ctx.accounts.observer.key();
    zero_copy_observation.observation_id = observation_id;
    zero_copy_observation.timestamp = timestamp;
    zero_copy_observation.value = value;
    zero_copy_observation.confidence = confidence;
    zero_copy_observation._padding = [0; 7];

    // Initialize zero-copy compression info as decompressed
    zero_copy_observation.zero_copy_compression_info = light_sdk::compressible::ZeroCopyCompressionInfo::some_decompressed()?;

    // Load and validate config
    let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;

    // Verify rent recipient matches config
    if ctx.accounts.rent_recipient.key() != config.rent_recipient {
        return err!(crate::ErrorCode::InvalidRentRecipient);
    }

    // Create CPI accounts
    let cpi_accounts = CpiAccounts::new(&ctx.accounts.observer, ctx.remaining_accounts, crate::LIGHT_CPI_SIGNER);

    let new_address_params = address_tree_info.into_new_address_params_packed(zero_copy_observation.key().to_bytes());

    // Handle compression manually for zero-copy types
    // Step 1: Set compression info to compressed state
    zero_copy_observation.zero_copy_compression_info.set_compressed()
        .map_err(|_| crate::ErrorCode::InvalidRentRecipient)?; // Convert error for now

    // Step 2: Create compressed account with data (compression_info set to none for storage)
    let owner_program_id = cpi_accounts.self_program_id();
    let mut compressed_account = light_sdk::account::sha::LightAccount::<'_, ZeroCopyObservation>::new_init(
        &owner_program_id,
        Some(compressed_address),
        output_state_tree_index,
    );

    // Clone the data and set zero-copy compression info to none for compressed storage
    let mut compressed_data = (**zero_copy_observation).clone(); // Double deref to get ZeroCopyObservation, then clone
    compressed_data.zero_copy_compression_info.set_none();
    compressed_account.account = compressed_data;

    // Step 3: Invoke CPI
    let compressed_infos = vec![compressed_account.to_account_info()?];
    let cpi_inputs = light_sdk::cpi::CpiInputs::new_with_address(proof, compressed_infos, vec![new_address_params]);
    cpi_inputs.invoke_light_system_program(cpi_accounts)?;

    // Step 4: Close the PDA account
    zero_copy_observation.close(ctx.accounts.rent_recipient.clone())?;

    Ok(())
}