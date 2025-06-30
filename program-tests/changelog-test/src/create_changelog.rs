use borsh::{BorshDeserialize, BorshSerialize};
use changelog::{Entry, GenericChangelog};
use light_account_checks::checks::account_info_init;
use light_sdk::{error::LightSdkError, LightDiscriminator};
use light_zero_copy::cyclic_vec::ZeroCopyCyclicVecU64;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    program::invoke_signed,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct CreateChangelogInstructionData {
    pub capacity: u64,
    pub bump: u8,
}

#[derive(LightDiscriminator)]
pub struct ChangelogAccount;

pub fn create_changelog<const WITH_CONTEXT: bool>(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    let mut instruction_data = instruction_data;
    let instruction_data = CreateChangelogInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| LightSdkError::Borsh)?;
    
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let changelog_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    
    // Derive the PDA for the changelog account
    let changelog_seed = b"changelog";
    let capacity_bytes = instruction_data.capacity.to_le_bytes();
    let (changelog_pubkey, bump) = Pubkey::find_program_address(
        &[changelog_seed, &capacity_bytes],
        &crate::ID,
    );
    
    // Verify the provided account matches our derived PDA
    if changelog_account.key != &changelog_pubkey {
        return Err(LightSdkError::ConstraintViolation);
    }
    
    if instruction_data.bump != bump {
        return Err(LightSdkError::ConstraintViolation);
    }
    
    // Calculate required space for the changelog (8 bytes discriminator + changelog data)
    let changelog_data_size = ZeroCopyCyclicVecU64::<Entry>::required_size_for_capacity(instruction_data.capacity);
    let required_size = 8 + changelog_data_size; // 8 bytes for discriminator
    let rent = Rent::get()?;
    let required_lamports = rent.minimum_balance(required_size);
    
    // Create the account
    let create_account_instruction = system_instruction::create_account(
        payer.key,
        changelog_account.key,
        required_lamports,
        required_size as u64,
        &crate::ID,
    );
    
    invoke_signed(
        &create_account_instruction,
        &[payer.clone(), changelog_account.clone(), system_program.clone()],
        &[&[changelog_seed, &capacity_bytes, &[bump]]],
    )?;
    
    // Set the discriminator
    account_info_init::<ChangelogAccount, _>(changelog_account)
        .map_err(|e| LightSdkError::ProgramError(e.into()))?;
    
    // Initialize the changelog with the specified capacity
    let mut account_data = changelog_account.try_borrow_mut_data()?;
    let _changelog = GenericChangelog::<Entry>::new(instruction_data.capacity, &mut account_data[8..])?;
    
    Ok(())
}