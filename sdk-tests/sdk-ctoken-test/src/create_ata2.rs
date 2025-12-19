use borsh::{BorshDeserialize, BorshSerialize};
use light_token_sdk::token::{CompressibleParamsCpi, CreateAssociatedTokenAccountCpi};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::{ATA_SEED, ID};

/// Instruction data for create ATA V2 (owner/mint as accounts)
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateAta2Data {
    pub bump: u8,
    pub pre_pay_num_epochs: u8,
    pub lamports_per_write: u32,
}

/// Handler for creating ATA using V2 variant (invoke)
///
/// Account order:
/// - accounts[0]: owner (readonly)
/// - accounts[1]: mint (readonly)
/// - accounts[2]: payer (signer, writable)
/// - accounts[3]: associated_token_account (writable)
/// - accounts[4]: system_program
/// - accounts[5]: compressible_config
/// - accounts[6]: rent_sponsor (writable)
pub fn process_create_ata2_invoke(
    accounts: &[AccountInfo],
    data: CreateAta2Data,
) -> Result<(), ProgramError> {
    if accounts.len() < 7 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let compressible_params = CompressibleParamsCpi::new(
        accounts[5].clone(),
        accounts[6].clone(),
        accounts[4].clone(),
    );

    CreateAssociatedTokenAccountCpi {
        owner: accounts[0].clone(),
        mint: accounts[1].clone(),
        payer: accounts[2].clone(),
        associated_token_account: accounts[3].clone(),
        system_program: accounts[4].clone(),
        bump: data.bump,
        compressible: Some(compressible_params),
        idempotent: false,
    }
    .invoke()?;

    Ok(())
}

/// Handler for creating ATA using V2 variant with PDA ownership (invoke_signed)
///
/// Account order:
/// - accounts[0]: owner (PDA, readonly)
/// - accounts[1]: mint (readonly)
/// - accounts[2]: payer (PDA, writable, not signer - program signs)
/// - accounts[3]: associated_token_account (writable)
/// - accounts[4]: system_program
/// - accounts[5]: compressible_config
/// - accounts[6]: rent_sponsor (writable)
pub fn process_create_ata2_invoke_signed(
    accounts: &[AccountInfo],
    data: CreateAta2Data,
) -> Result<(), ProgramError> {
    if accounts.len() < 7 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA that will act as payer
    let (pda, bump) = Pubkey::find_program_address(&[ATA_SEED], &ID);

    // Verify the payer is the PDA
    if &pda != accounts[2].key {
        return Err(ProgramError::InvalidSeeds);
    }

    let compressible_params = CompressibleParamsCpi::new(
        accounts[5].clone(),
        accounts[6].clone(),
        accounts[4].clone(),
    );

    let signer_seeds: &[&[u8]] = &[ATA_SEED, &[bump]];
    CreateAssociatedTokenAccountCpi {
        owner: accounts[0].clone(),
        mint: accounts[1].clone(),
        payer: accounts[2].clone(), // PDA
        associated_token_account: accounts[3].clone(),
        system_program: accounts[4].clone(),
        bump: data.bump,
        compressible: Some(compressible_params),
        idempotent: false,
    }
    .invoke_signed(&[signer_seeds])?;

    Ok(())
}
