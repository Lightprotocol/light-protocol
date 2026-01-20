use borsh::{BorshDeserialize, BorshSerialize};
use light_token::instruction::CreateTokenAtaCpi;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::{ATA_SEED, ID};

/// Instruction data for create ATA (owner and mint passed as accounts)
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateAtaData {
    pub bump: u8,
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
    data: CreateAtaData,
) -> Result<(), ProgramError> {
    if accounts.len() < 7 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    CreateTokenAtaCpi {
        payer: accounts[2].clone(),
        owner: accounts[0].clone(),
        mint: accounts[1].clone(),
        ata: accounts[3].clone(),
        bump: data.bump,
    }
    .rent_free(
        accounts[5].clone(), // compressible_config
        accounts[6].clone(), // rent_sponsor
        accounts[4].clone(), // system_program
    )
    .invoke()?;

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
    data: CreateAtaData,
) -> Result<(), ProgramError> {
    if accounts.len() < 7 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA that will act as payer/owner
    let (pda, bump) = Pubkey::find_program_address(&[ATA_SEED], &ID);

    // Verify the payer is the PDA
    if &pda != accounts[2].key {
        return Err(ProgramError::InvalidSeeds);
    }

    let signer_seeds: &[&[u8]] = &[ATA_SEED, &[bump]];

    CreateTokenAtaCpi {
        payer: accounts[2].clone(),
        owner: accounts[0].clone(),
        mint: accounts[1].clone(),
        ata: accounts[3].clone(),
        bump: data.bump,
    }
    .rent_free(
        accounts[5].clone(), // compressible_config
        accounts[6].clone(), // rent_sponsor
        accounts[4].clone(), // system_program
    )
    .invoke_signed(&[signer_seeds])?;

    Ok(())
}
