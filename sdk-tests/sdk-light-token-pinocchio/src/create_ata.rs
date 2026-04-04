use borsh::{BorshDeserialize, BorshSerialize};
use light_token_pinocchio::instruction::{CompressibleParamsCpi, CreateTokenAtaCpi};
use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    AccountView as AccountInfo, Address,
};

use crate::{ATA_SEED, ID};

/// Instruction data for create ATA (owner and mint passed as accounts)
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateAtaData {
    pub pre_pay_num_epochs: u8,
    pub lamports_per_write: u32,
}

/// Handler for creating a compressible associated token account (invoke)
///
/// Account order:
/// - accounts[0]: owner
/// - accounts[1]: mint
/// - accounts[2]: payer (signer)
/// - accounts[3]: associated token account (derived)
/// - accounts[4]: system_program
/// - accounts[5]: compressible_config
/// - accounts[6]: rent_sponsor
pub fn process_create_ata_invoke(
    accounts: &[AccountInfo],
    _data: CreateAtaData,
) -> Result<(), ProgramError> {
    if accounts.len() < 7 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    CreateTokenAtaCpi {
        payer: &accounts[2],
        owner: &accounts[0],
        mint: &accounts[1],
        ata: &accounts[3],
    }
    .rent_free(
        &accounts[5], // compressible_config
        &accounts[6], // rent_sponsor
        &accounts[4], // system_program
    )
    .invoke()
    .map_err(|_| ProgramError::Custom(0))?;

    Ok(())
}

/// Handler for creating a compressible ATA with PDA ownership (invoke_signed)
///
/// Account order:
/// - accounts[0]: owner
/// - accounts[1]: mint
/// - accounts[2]: payer (PDA, signer via invoke_signed)
/// - accounts[3]: associated token account (derived)
/// - accounts[4]: system_program
/// - accounts[5]: compressible_config
/// - accounts[6]: rent_sponsor
pub fn process_create_ata_invoke_signed(
    accounts: &[AccountInfo],
    _data: CreateAtaData,
) -> Result<(), ProgramError> {
    if accounts.len() < 7 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA that will act as payer/owner
    let (pda, bump) = Address::find_program_address(&[ATA_SEED], &Address::from(ID));

    // Verify the payer is the PDA
    if pda != *accounts[2].address() {
        return Err(ProgramError::InvalidSeeds);
    }

    let bump_byte = [bump];
    let seeds = [Seed::from(ATA_SEED), Seed::from(&bump_byte[..])];
    let signer = Signer::from(&seeds);

    CreateTokenAtaCpi {
        payer: &accounts[2],
        owner: &accounts[0],
        mint: &accounts[1],
        ata: &accounts[3],
    }
    .rent_free(
        &accounts[5], // compressible_config
        &accounts[6], // rent_sponsor
        &accounts[4], // system_program
    )
    .invoke_signed(&[signer])
    .map_err(|_| ProgramError::Custom(0))?;

    Ok(())
}

/// Handler for creating a compressible ATA using invoke_with (explicit CompressibleParamsCpi).
///
/// Account order:
/// - accounts[0]: owner
/// - accounts[1]: mint
/// - accounts[2]: payer (signer)
/// - accounts[3]: associated token account (derived)
/// - accounts[4]: system_program
/// - accounts[5]: compressible_config
/// - accounts[6]: rent_sponsor
pub fn process_create_ata_invoke_with(
    accounts: &[AccountInfo],
    _data: CreateAtaData,
) -> Result<(), ProgramError> {
    if accounts.len() < 7 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let compressible = CompressibleParamsCpi::new_ata(
        &accounts[5], // compressible_config
        &accounts[6], // rent_sponsor
        &accounts[4], // system_program
    );

    CreateTokenAtaCpi {
        payer: &accounts[2],
        owner: &accounts[0],
        mint: &accounts[1],
        ata: &accounts[3],
    }
    .invoke_with(compressible)
    .map_err(|_| ProgramError::Custom(0))?;

    Ok(())
}

/// Handler for creating a compressible ATA idempotently using idempotent().invoke_with().
///
/// Account order:
/// - accounts[0]: owner
/// - accounts[1]: mint
/// - accounts[2]: payer (signer)
/// - accounts[3]: associated token account (derived)
/// - accounts[4]: system_program
/// - accounts[5]: compressible_config
/// - accounts[6]: rent_sponsor
pub fn process_create_ata_idempotent_invoke_with(
    accounts: &[AccountInfo],
    _data: CreateAtaData,
) -> Result<(), ProgramError> {
    if accounts.len() < 7 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let compressible = CompressibleParamsCpi::new_ata(
        &accounts[5], // compressible_config
        &accounts[6], // rent_sponsor
        &accounts[4], // system_program
    );

    CreateTokenAtaCpi {
        payer: &accounts[2],
        owner: &accounts[0],
        mint: &accounts[1],
        ata: &accounts[3],
    }
    .idempotent()
    .invoke_with(compressible)
    .map_err(|_| ProgramError::Custom(0))?;

    Ok(())
}
