use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_token_sdk::ctoken::{
    CompressibleParamsInfos, CreateAssociatedTokenAccountInfos,
};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::{ATA_SEED, ID};

/// Instruction data for create ATA
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateAtaData {
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub bump: u8,
    pub pre_pay_num_epochs: u8,
    pub lamports_per_write: u32,
}

/// Handler for creating a compressible associated token account (invoke)
///
/// Account order:
/// - accounts[0]: payer (signer)
/// - accounts[1]: associated token account (derived)
/// - accounts[2]: system_program
/// - accounts[3]: compressible_config
/// - accounts[4]: rent_sponsor
pub fn process_create_ata_invoke(
    accounts: &[AccountInfo],
    data: CreateAtaData,
) -> Result<(), ProgramError> {
    if accounts.len() < 5 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Build the compressible params using constructor
    let compressible_params = CompressibleParamsInfos::new(
        accounts[3].clone(),
        accounts[4].clone(),
        accounts[2].clone(),
    );

    // Use the CreateAssociatedTokenAccountInfos constructor
    CreateAssociatedTokenAccountInfos::new(
        data.bump,
        data.owner,
        data.mint,
        accounts[0].clone(),
        accounts[1].clone(),
        accounts[2].clone(),
        compressible_params,
    )
    .invoke()?;

    Ok(())
}

/// Handler for creating a compressible ATA with PDA ownership (invoke_signed)
///
/// Account order:
/// - accounts[0]: payer (PDA, signer via invoke_signed)
/// - accounts[1]: associated token account (derived)
/// - accounts[2]: system_program
/// - accounts[3]: compressible_config
/// - accounts[4]: rent_sponsor
pub fn process_create_ata_invoke_signed(
    accounts: &[AccountInfo],
    data: CreateAtaData,
) -> Result<(), ProgramError> {
    if accounts.len() < 5 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Derive the PDA that will act as payer/owner
    let (pda, bump) = Pubkey::find_program_address(&[ATA_SEED], &ID);

    // Verify the payer is the PDA
    if &pda != accounts[0].key {
        return Err(ProgramError::InvalidSeeds);
    }

    // Build the compressible params using constructor
    let compressible_params = CompressibleParamsInfos::new(
        accounts[3].clone(),
        accounts[4].clone(),
        accounts[2].clone(),
    );

    // Use the CreateAssociatedTokenAccountInfos constructor
    let account_infos = CreateAssociatedTokenAccountInfos::new(
        data.bump,
        data.owner,
        data.mint,
        accounts[0].clone(),
        accounts[1].clone(),
        accounts[2].clone(),
        compressible_params,
    );

    // Invoke with PDA signing
    let signer_seeds: &[&[u8]] = &[ATA_SEED, &[bump]];
    account_infos.invoke_signed(&[signer_seeds])?;

    Ok(())
}
