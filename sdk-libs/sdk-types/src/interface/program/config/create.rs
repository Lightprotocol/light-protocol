//! Config initialization instructions (generic over AccountInfoTrait).

use alloc::vec::Vec;

use light_account_checks::{
    checks::check_signer,
    discriminator::{Discriminator, DISCRIMINATOR_LEN},
    AccountInfoTrait,
};
use light_compressible::rent::RentConfig;

use super::{state::LightConfig, validate_address_space_no_duplicates, COMPRESSIBLE_CONFIG_SEED};
use crate::{error::LightSdkTypesError, AnchorSerialize};

/// BPFLoaderUpgradeab1e11111111111111111111111 as raw bytes.
const BPF_LOADER_UPGRADEABLE_ID: [u8; 32] = [
    2, 168, 246, 145, 78, 136, 161, 110, 57, 90, 225, 40, 148, 143, 144, 16, 207, 227, 47, 228,
    248, 212, 16, 185, 221, 165, 30, 160, 42, 103, 43, 122,
];

/// UpgradeableLoaderState::ProgramData layout (manual parsing, no bincode dep):
/// - bytes 0..4:  variant tag (u32 LE, must be 3 for ProgramData)
/// - bytes 4..12: slot (u64 LE)
/// - byte  12:    Option discriminant (0=None, 1=Some)
/// - bytes 13..45: authority pubkey (32 bytes, only valid when discriminant=1)
const PROGRAM_DATA_VARIANT_TAG: u32 = 3;
const PROGRAM_DATA_MIN_LEN: usize = 45;

/// Creates a new compressible config PDA.
///
/// # Required Validation (must be done by caller)
/// The caller MUST validate that the signer is the program's upgrade authority.
/// Use `process_initialize_light_config_checked` for the version that does this.
#[allow(clippy::too_many_arguments)]
pub fn process_initialize_light_config<AI: AccountInfoTrait + Clone>(
    config_account: &AI,
    update_authority: &AI,
    rent_sponsor: &[u8; 32],
    compression_authority: &[u8; 32],
    rent_config: RentConfig,
    write_top_up: u32,
    address_space: Vec<[u8; 32]>,
    config_bump: u8,
    payer: &AI,
    system_program: &AI,
    program_id: &[u8; 32],
) -> Result<(), LightSdkTypesError> {
    // CHECK: config_bump must be 0
    if config_bump != 0 {
        return Err(LightSdkTypesError::ConstraintViolation);
    }

    // CHECK: not already initialized
    if !config_account.data_is_empty() {
        return Err(LightSdkTypesError::ConstraintViolation);
    }

    // CHECK: exactly 1 address space
    if address_space.len() != 1 {
        return Err(LightSdkTypesError::ConstraintViolation);
    }

    // CHECK: unique pubkeys in address_space
    validate_address_space_no_duplicates(&address_space)?;

    // CHECK: signer
    check_signer(update_authority).map_err(LightSdkTypesError::AccountError)?;

    // CHECK: PDA derivation
    let (derived_pda, bump) = LightConfig::derive_pda_bytes::<AI>(program_id, config_bump);
    if derived_pda != config_account.key() {
        return Err(LightSdkTypesError::ConstraintViolation);
    }

    // Derive rent_sponsor_bump for storage
    let (derived_rent_sponsor, rent_sponsor_bump) =
        LightConfig::derive_rent_sponsor_pda_bytes::<AI>(program_id);
    if *rent_sponsor != derived_rent_sponsor {
        return Err(LightSdkTypesError::InvalidRentSponsor);
    }

    let account_size = LightConfig::size_for_address_space(address_space.len());
    let rent_lamports =
        AI::get_min_rent_balance(account_size).map_err(LightSdkTypesError::AccountError)?;

    // Create PDA using AccountInfoTrait
    let config_bump_bytes = (config_bump as u16).to_le_bytes();
    let seeds: &[&[u8]] = &[
        COMPRESSIBLE_CONFIG_SEED,
        config_bump_bytes.as_ref(),
        &[bump],
    ];

    config_account.create_pda_account(
        rent_lamports,
        account_size as u64,
        program_id,
        seeds,
        payer,
        &[],
        system_program,
    )?;

    let config = LightConfig {
        version: 1,
        write_top_up,
        update_authority: update_authority.key(),
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
        .map_err(LightSdkTypesError::AccountError)?;

    // Write discriminator first
    data[..DISCRIMINATOR_LEN].copy_from_slice(&LightConfig::LIGHT_DISCRIMINATOR);

    // Serialize config data after discriminator
    config
        .serialize(&mut &mut data[DISCRIMINATOR_LEN..])
        .map_err(|_| LightSdkTypesError::Borsh)?;

    Ok(())
}

/// Checks that the signer is the program's upgrade authority.
///
/// Manually parses the UpgradeableLoaderState::ProgramData layout (45 bytes)
/// to avoid a bincode dependency.
pub fn check_program_upgrade_authority<AI: AccountInfoTrait>(
    program_id: &[u8; 32],
    program_data_account: &AI,
    authority: &AI,
) -> Result<(), LightSdkTypesError> {
    // CHECK: program data PDA
    let (expected_program_data, _) =
        AI::find_program_address(&[program_id], &BPF_LOADER_UPGRADEABLE_ID);
    if program_data_account.key() != expected_program_data {
        return Err(LightSdkTypesError::ConstraintViolation);
    }

    let data = program_data_account
        .try_borrow_data()
        .map_err(LightSdkTypesError::AccountError)?;

    if data.len() < PROGRAM_DATA_MIN_LEN {
        return Err(LightSdkTypesError::AccountDataTooSmall);
    }

    // Parse variant tag (4 bytes, u32 LE)
    let variant_tag = u32::from_le_bytes(data[0..4].try_into().unwrap());
    if variant_tag != PROGRAM_DATA_VARIANT_TAG {
        return Err(LightSdkTypesError::ConstraintViolation);
    }

    // Parse Option<Pubkey> at offset 12
    let option_discriminant = data[12];
    let upgrade_authority: [u8; 32] = match option_discriminant {
        0 => {
            // None - program has no upgrade authority
            return Err(LightSdkTypesError::ConstraintViolation);
        }
        1 => {
            let mut auth = [0u8; 32];
            auth.copy_from_slice(&data[13..45]);
            // Check for invalid zero authority
            if auth == [0u8; 32] {
                return Err(LightSdkTypesError::ConstraintViolation);
            }
            auth
        }
        _ => {
            return Err(LightSdkTypesError::ConstraintViolation);
        }
    };

    // CHECK: authority is signer
    check_signer(authority).map_err(LightSdkTypesError::AccountError)?;

    // CHECK: authority matches upgrade authority
    if authority.key() != upgrade_authority {
        return Err(LightSdkTypesError::ConstraintViolation);
    }

    Ok(())
}

/// Creates a new compressible config PDA with upgrade authority check.
#[allow(clippy::too_many_arguments)]
pub fn process_initialize_light_config_checked<AI: AccountInfoTrait + Clone>(
    config_account: &AI,
    update_authority: &AI,
    program_data_account: &AI,
    rent_sponsor: &[u8; 32],
    compression_authority: &[u8; 32],
    rent_config: RentConfig,
    write_top_up: u32,
    address_space: Vec<[u8; 32]>,
    config_bump: u8,
    payer: &AI,
    system_program: &AI,
    program_id: &[u8; 32],
) -> Result<(), LightSdkTypesError> {
    check_program_upgrade_authority::<AI>(program_id, program_data_account, update_authority)?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upgradeable_loader_state_parsing() {
        // Build a synthetic ProgramData account matching the manual layout
        let mut data = [0u8; 45];

        // Variant tag = 3 (ProgramData)
        data[0..4].copy_from_slice(&3u32.to_le_bytes());

        // Slot = 42
        data[4..12].copy_from_slice(&42u64.to_le_bytes());

        // Option discriminant = 1 (Some)
        data[12] = 1;

        // Authority pubkey = [1..=32]
        let authority: [u8; 32] = core::array::from_fn(|i| (i + 1) as u8);
        data[13..45].copy_from_slice(&authority);

        // Parse variant tag
        let tag = u32::from_le_bytes(data[0..4].try_into().unwrap());
        assert_eq!(tag, PROGRAM_DATA_VARIANT_TAG);

        // Parse slot
        let slot = u64::from_le_bytes(data[4..12].try_into().unwrap());
        assert_eq!(slot, 42);

        // Parse authority
        assert_eq!(data[12], 1);
        let mut parsed_auth = [0u8; 32];
        parsed_auth.copy_from_slice(&data[13..45]);
        assert_eq!(parsed_auth, authority);

        // Test None case
        data[12] = 0;
        assert_eq!(data[12], 0);
    }
}
