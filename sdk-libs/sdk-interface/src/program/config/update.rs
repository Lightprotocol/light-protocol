//! Config update instruction (generic over AccountInfoTrait).

use light_account_checks::{checks::check_signer, discriminator::DISCRIMINATOR_LEN, AccountInfoTrait};
use light_compressible::rent::RentConfig;

use super::{
    state::LightConfig, validate_address_space_no_duplicates, validate_address_space_only_adds,
    MAX_ADDRESS_TREES_PER_SPACE,
};
use crate::{error::LightPdaError, AnchorSerialize};

/// Updates an existing compressible config.
#[allow(clippy::too_many_arguments)]
pub fn process_update_light_config<AI: AccountInfoTrait>(
    config_account: &AI,
    authority: &AI,
    new_update_authority: Option<&[u8; 32]>,
    new_rent_sponsor: Option<&[u8; 32]>,
    new_compression_authority: Option<&[u8; 32]>,
    new_rent_config: Option<RentConfig>,
    new_write_top_up: Option<u32>,
    new_address_space: Option<Vec<[u8; 32]>>,
    owner_program_id: &[u8; 32],
) -> Result<(), LightPdaError> {
    // CHECK: PDA derivation + discriminator + owner
    let mut config = LightConfig::load_checked(config_account, owner_program_id)?;

    // CHECK: signer
    check_signer(authority).map_err(LightPdaError::AccountCheck)?;

    // CHECK: authority
    if authority.key() != config.update_authority {
        return Err(LightPdaError::ConstraintViolation);
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
            return Err(LightPdaError::ConstraintViolation);
        }

        validate_address_space_no_duplicates(&new_address_space)?;
        validate_address_space_only_adds(&config.address_space, &new_address_space)?;

        config.address_space = new_address_space;
    }

    let mut data = config_account
        .try_borrow_mut_data()
        .map_err(LightPdaError::AccountCheck)?;
    // Serialize after discriminator (discriminator is preserved from init)
    config
        .serialize(&mut &mut data[DISCRIMINATOR_LEN..])
        .map_err(|_| LightPdaError::Borsh)?;

    Ok(())
}
