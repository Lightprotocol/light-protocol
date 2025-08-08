use anchor_lang::solana_program::{
    program_error::ProgramError, rent::Rent, system_instruction, sysvar::Sysvar,
};
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
};

/// Configuration for creating a PDA account
pub struct CreatePdaAccountConfig<'a> {
    /// The seeds used to derive the PDA (without bump)
    pub seeds: &'a [&'a [u8]],
    /// The bump seed for PDA derivation
    pub bump: u8,
    /// Size of the account in bytes
    pub account_size: usize,
    /// Program that will own the created account
    pub owner_program_id: &'a pinocchio::pubkey::Pubkey,
    /// Program used to derive the PDA (usually our program ID)
    pub derivation_program_id: &'a pinocchio::pubkey::Pubkey,
}

/// Creates a PDA account with the specified configuration
///
/// This function handles the common pattern of:
/// 1. Calculating rent for the account size
/// 2. Creating seeds array with bump
/// 3. Creating the account via system program
/// 4. Signing with PDA seeds
pub fn create_pda_account(
    fee_payer: &AccountInfo,
    new_account: &AccountInfo,
    system_program: &AccountInfo,
    config: CreatePdaAccountConfig,
) -> Result<(), ProgramError> {
    // Calculate rent
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(config.account_size);

    // Build seeds array with bump
    let bump_bytes = [config.bump];
    let mut seed_vec: Vec<Seed> = config.seeds.iter().map(|&seed| Seed::from(seed)).collect();
    seed_vec.push(Seed::from(bump_bytes.as_ref()));
    let signer = Signer::from(seed_vec.as_slice());

    // Create the account
    let fee_payer_pubkey = solana_pubkey::Pubkey::new_from_array(*fee_payer.key());
    let new_account_pubkey = solana_pubkey::Pubkey::new_from_array(*new_account.key());
    let owner_program_pubkey = solana_pubkey::Pubkey::new_from_array(*config.owner_program_id);

    let create_account_ix = system_instruction::create_account(
        &fee_payer_pubkey,
        &new_account_pubkey,
        lamports,
        config.account_size as u64,
        &owner_program_pubkey,
    );

    let pinocchio_instruction = pinocchio::instruction::Instruction {
        program_id: &create_account_ix.program_id.to_bytes(),
        accounts: &[
            pinocchio::instruction::AccountMeta::new(fee_payer.key(), true, true),
            pinocchio::instruction::AccountMeta::new(new_account.key(), true, true),
            pinocchio::instruction::AccountMeta::readonly(system_program.key()),
        ],
        data: &create_account_ix.data,
    };

    match pinocchio::program::invoke_signed(
        &pinocchio_instruction,
        &[fee_payer, new_account, system_program],
        &[signer],
    ) {
        Ok(()) => Ok(()),
        Err(e) => Err(ProgramError::Custom(u64::from(e) as u32)),
    }
}

/// Verifies that the provided account matches the expected PDA
pub fn verify_pda(
    account_key: &[u8; 32],
    seeds: &[&[u8]],
    bump: u8,
    program_id: &pinocchio::pubkey::Pubkey,
) -> Result<(), ProgramError> {
    let program_id_pubkey = solana_pubkey::Pubkey::new_from_array(*program_id);
    let mut seeds_with_bump: Vec<&[u8]> = seeds.to_vec();
    let bump_bytes = [bump];
    seeds_with_bump.push(&bump_bytes);
    
    let expected_pubkey = solana_pubkey::Pubkey::create_program_address(&seeds_with_bump, &program_id_pubkey)
        .map_err(|_| ProgramError::InvalidAccountData)?;
    
    if account_key != &expected_pubkey.to_bytes() {
        return Err(ProgramError::InvalidAccountData);
    }
    
    Ok(())
}