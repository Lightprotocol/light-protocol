//! Config initialization instructions.

use light_account_checks::{
    checks::check_signer,
    discriminator::{Discriminator, DISCRIMINATOR_LEN},
};
use light_compressible::rent::RentConfig;
use solana_account_info::AccountInfo;
use solana_cpi::invoke_signed;
use solana_loader_v3_interface::state::UpgradeableLoaderState;
use solana_msg::msg;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;
use solana_system_interface::instruction as system_instruction;
use solana_sysvar::{rent::Rent, Sysvar};

use super::{state::LightConfig, validate_address_space_no_duplicates, COMPRESSIBLE_CONFIG_SEED};
use crate::{error::LightPdaError, AnchorSerialize};

const BPF_LOADER_UPGRADEABLE_ID: Pubkey =
    Pubkey::from_str_const("BPFLoaderUpgradeab1e11111111111111111111111");

/// Creates a new compressible config PDA
///
/// # Security - Solana Best Practice
/// This function follows the standard Solana pattern where only the program's
/// upgrade authority can create the initial config. This prevents unauthorized
/// parties from hijacking the config system.
///
/// # Arguments
/// * `config_account` - The config PDA account to initialize
/// * `update_authority` - Authority that can update the config after creation
/// * `rent_sponsor` - Account that receives rent from compressed PDAs
/// * `compression_authority` - Authority that can compress/close PDAs
/// * `rent_config` - Rent function parameters
/// * `write_top_up` - Lamports to top up on each write
/// * `address_space` - Address space for compressed accounts (currently 1 address_tree allowed)
/// * `config_bump` - Config bump seed (must be 0 for now)
/// * `payer` - Account paying for the PDA creation
/// * `system_program` - System program
/// * `program_id` - The program that owns the config
///
/// # Required Validation (must be done by caller)
/// The caller MUST validate that the signer is the program's upgrade authority
/// by checking against the program data account. This cannot be done in the SDK
/// due to dependency constraints.
///
/// # Returns
/// * `Ok(())` if config was created successfully
/// * `Err(ProgramError)` if there was an error
#[allow(clippy::too_many_arguments)]
pub fn process_initialize_light_config<'info>(
    config_account: &AccountInfo<'info>,
    update_authority: &AccountInfo<'info>,
    rent_sponsor: &Pubkey,
    compression_authority: &Pubkey,
    rent_config: RentConfig,
    write_top_up: u32,
    address_space: Vec<Pubkey>,
    config_bump: u8,
    payer: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    program_id: &Pubkey,
) -> Result<(), ProgramError> {
    // CHECK: only 1 address_space
    if config_bump != 0 {
        msg!("Config bump must be 0 for now, found: {}", config_bump);
        return Err(LightPdaError::ConstraintViolation.into());
    }

    // CHECK: not already initialized
    if config_account.data_len() > 0 {
        msg!("Config account already initialized");
        return Err(LightPdaError::ConstraintViolation.into());
    }

    // CHECK: only 1 address_space
    if address_space.len() != 1 {
        msg!(
            "Address space must contain exactly 1 pubkey, found: {}",
            address_space.len()
        );
        return Err(LightPdaError::ConstraintViolation.into());
    }

    // CHECK: unique pubkeys in address_space
    validate_address_space_no_duplicates(&address_space)?;

    // CHECK: signer
    check_signer(update_authority).inspect_err(|_| {
        msg!("Update authority must be signer for initial config creation");
    })?;

    // CHECK: pda derivation
    let (derived_pda, bump) = LightConfig::derive_pda(program_id, config_bump);
    if derived_pda != *config_account.key {
        msg!("Invalid config PDA");
        return Err(LightPdaError::ConstraintViolation.into());
    }

    // Derive rent_sponsor_bump for storage
    let (derived_rent_sponsor, rent_sponsor_bump) =
        LightConfig::derive_rent_sponsor_pda(program_id);
    if *rent_sponsor != derived_rent_sponsor {
        msg!(
            "rent_sponsor must be derived PDA: expected {:?}, got {:?}",
            derived_rent_sponsor,
            rent_sponsor
        );
        return Err(LightPdaError::InvalidRentSponsor.into());
    }

    let rent = Rent::get().map_err(LightPdaError::from)?;
    let account_size = LightConfig::size_for_address_space(address_space.len());
    let rent_lamports = rent.minimum_balance(account_size);

    // Use u16 to_le_bytes to match derive_pda (2 bytes instead of 1)
    let config_bump_bytes = (config_bump as u16).to_le_bytes();
    let seeds = &[
        COMPRESSIBLE_CONFIG_SEED,
        config_bump_bytes.as_ref(),
        &[bump],
    ];
    let create_account_ix = system_instruction::create_account(
        payer.key,
        config_account.key,
        rent_lamports,
        account_size as u64,
        program_id,
    );

    invoke_signed(
        &create_account_ix,
        &[
            payer.clone(),
            config_account.clone(),
            system_program.clone(),
        ],
        &[seeds],
    )
    .map_err(LightPdaError::from)?;

    let config = LightConfig {
        version: 1,
        write_top_up,
        update_authority: *update_authority.key,
        rent_sponsor: *rent_sponsor,
        compression_authority: *compression_authority,
        rent_config,
        config_bump,
        bump,
        rent_sponsor_bump,
        address_space,
    };

    let mut data = config_account
        .try_borrow_mut_data()
        .map_err(LightPdaError::from)?;

    // Write discriminator first (using trait constant)
    data[..DISCRIMINATOR_LEN].copy_from_slice(&LightConfig::LIGHT_DISCRIMINATOR);

    // Serialize config data after discriminator
    config
        .serialize(&mut &mut data[DISCRIMINATOR_LEN..])
        .map_err(|_| LightPdaError::Borsh)?;

    Ok(())
}

/// Checks that the signer is the program's upgrade authority
///
/// # Arguments
/// * `program_id` - The program to check
/// * `program_data_account` - The program's data account (ProgramData)
/// * `authority` - The authority to verify
///
/// # Returns
/// * `Ok(())` if authority is valid
/// * `Err(LightPdaError)` if authority is invalid or verification fails
pub fn check_program_upgrade_authority(
    program_id: &Pubkey,
    program_data_account: &AccountInfo,
    authority: &AccountInfo,
) -> Result<(), ProgramError> {
    // CHECK: program data PDA
    let (expected_program_data, _) =
        Pubkey::find_program_address(&[program_id.as_ref()], &BPF_LOADER_UPGRADEABLE_ID);
    if program_data_account.key != &expected_program_data {
        msg!("Invalid program data account");
        return Err(LightPdaError::ConstraintViolation.into());
    }

    let data = program_data_account.try_borrow_data()?;
    let program_state: UpgradeableLoaderState = bincode::deserialize(&data).map_err(|_| {
        msg!("Failed to deserialize program data account");
        LightPdaError::ConstraintViolation
    })?;

    // Extract upgrade authority
    let upgrade_authority = match program_state {
        UpgradeableLoaderState::ProgramData {
            slot: _,
            upgrade_authority_address,
        } => {
            match upgrade_authority_address {
                Some(auth) => {
                    // Check for invalid zero authority when authority exists
                    if auth == Pubkey::default() {
                        msg!("Invalid state: authority is zero pubkey");
                        return Err(LightPdaError::ConstraintViolation.into());
                    }
                    auth
                }
                None => {
                    msg!("Program has no upgrade authority");
                    return Err(LightPdaError::ConstraintViolation.into());
                }
            }
        }
        _ => {
            msg!("Account is not ProgramData, found: {:?}", program_state);
            return Err(LightPdaError::ConstraintViolation.into());
        }
    };

    // CHECK: upgrade authority is signer
    check_signer(authority).inspect_err(|_| {
        msg!("Authority must be signer");
    })?;

    // CHECK: upgrade authority is program's upgrade authority
    if *authority.key != upgrade_authority {
        msg!(
            "Signer is not the program's upgrade authority. Signer: {:?}, Expected Authority: {:?}",
            authority.key,
            upgrade_authority
        );
        return Err(LightPdaError::ConstraintViolation.into());
    }

    Ok(())
}

/// Creates a new compressible config PDA.
///
/// # Arguments
/// * `config_account` - The config PDA account to initialize
/// * `update_authority` - Must be the program's upgrade authority
/// * `program_data_account` - The program's data account for validation
/// * `rent_sponsor` - Account that receives rent from compressed PDAs
/// * `compression_authority` - Authority that can compress/close PDAs
/// * `rent_config` - Rent function parameters
/// * `write_top_up` - Lamports to top up on each write
/// * `address_space` - Address spaces for compressed accounts (exactly 1
///   allowed)
/// * `config_bump` - Config bump seed (must be 0 for now)
/// * `payer` - Account paying for the PDA creation
/// * `system_program` - System program
/// * `program_id` - The program that owns the config
///
/// # Returns
/// * `Ok(())` if config was created successfully
/// * `Err(ProgramError)` if there was an error or authority validation fails
#[allow(clippy::too_many_arguments)]
pub fn process_initialize_light_config_checked<'info>(
    config_account: &AccountInfo<'info>,
    update_authority: &AccountInfo<'info>,
    program_data_account: &AccountInfo<'info>,
    rent_sponsor: &Pubkey,
    compression_authority: &Pubkey,
    rent_config: RentConfig,
    write_top_up: u32,
    address_space: Vec<Pubkey>,
    config_bump: u8,
    payer: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    program_id: &Pubkey,
) -> Result<(), ProgramError> {
    msg!(
        "create_compression_config_checked program_data_account: {:?}",
        program_data_account.key
    );
    msg!(
        "create_compression_config_checked program_id: {:?}",
        program_id
    );
    // Verify the signer is the program's upgrade authority
    check_program_upgrade_authority(program_id, program_data_account, update_authority)?;

    // Create the config with validated authority
    process_initialize_light_config(
        config_account,
        update_authority,
        rent_sponsor,
        compression_authority,
        rent_config,
        write_top_up,
        address_space,
        config_bump,
        payer,
        system_program,
        program_id,
    )
}
