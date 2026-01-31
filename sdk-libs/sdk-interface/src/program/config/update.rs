//! Config update instruction.

use light_account_checks::{checks::check_signer, discriminator::DISCRIMINATOR_LEN};
use light_compressible::rent::RentConfig;
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use super::{
    state::LightConfig, validate_address_space_no_duplicates, validate_address_space_only_adds,
    MAX_ADDRESS_TREES_PER_SPACE,
};
use crate::{error::LightPdaError, AnchorSerialize};

/// Updates an existing compressible config
///
/// # Arguments
/// * `config_account` - The config PDA account to update
/// * `authority` - Current update authority (must match config)
/// * `new_update_authority` - Optional new update authority
/// * `new_rent_sponsor` - Optional new rent recipient
/// * `new_compression_authority` - Optional new compression authority
/// * `new_rent_config` - Optional new rent function parameters
/// * `new_write_top_up` - Optional new write top-up amount
/// * `new_address_space` - Optional new address space (currently 1 address_tree allowed)
/// * `owner_program_id` - The program that owns the config
///
/// # Returns
/// * `Ok(())` if config was updated successfully
/// * `Err(ProgramError)` if there was an error
#[allow(clippy::too_many_arguments)]
pub fn process_update_light_config<'info>(
    config_account: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    new_update_authority: Option<&Pubkey>,
    new_rent_sponsor: Option<&Pubkey>,
    new_compression_authority: Option<&Pubkey>,
    new_rent_config: Option<RentConfig>,
    new_write_top_up: Option<u32>,
    new_address_space: Option<Vec<Pubkey>>,
    owner_program_id: &Pubkey,
) -> Result<(), ProgramError> {
    // CHECK: PDA derivation
    let mut config = LightConfig::load_checked(config_account, owner_program_id)?;

    // CHECK: signer
    check_signer(authority).inspect_err(|_| {
        msg!("Update authority must be signer");
    })?;
    // CHECK: authority
    if *authority.key != config.update_authority {
        msg!("Invalid update authority");
        return Err(LightPdaError::ConstraintViolation.into());
    }

    if let Some(new_authority) = new_update_authority {
        config.update_authority = *new_authority;
    }
    if let Some(new_recipient) = new_rent_sponsor {
        config.rent_sponsor = *new_recipient;
    }
    if let Some(new_auth) = new_compression_authority {
        config.compression_authority = *new_auth;
    }
    if let Some(new_rcfg) = new_rent_config {
        config.rent_config = new_rcfg;
    }
    if let Some(new_top_up) = new_write_top_up {
        config.write_top_up = new_top_up;
    }
    if let Some(new_address_space) = new_address_space {
        // CHECK: address space length
        if new_address_space.len() != MAX_ADDRESS_TREES_PER_SPACE {
            msg!(
                "New address space must contain exactly 1 pubkey, found: {}",
                new_address_space.len()
            );
            return Err(LightPdaError::ConstraintViolation.into());
        }

        validate_address_space_no_duplicates(&new_address_space)?;

        validate_address_space_only_adds(&config.address_space, &new_address_space)?;

        config.address_space = new_address_space;
    }

    let mut data = config_account.try_borrow_mut_data().map_err(|e| {
        msg!("Failed to borrow mut data for config_account: {:?}", e);
        LightPdaError::from(e)
    })?;
    // Serialize after discriminator (discriminator is preserved from init)
    config
        .serialize(&mut &mut data[DISCRIMINATOR_LEN..])
        .map_err(|e| {
            msg!("Failed to serialize updated config: {:?}", e);
            LightPdaError::Borsh
        })?;

    Ok(())
}
