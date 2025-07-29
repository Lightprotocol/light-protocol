use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::{
    compressible::{compress_account_on_init_native, CompressibleConfig, CompressionInfo},
    cpi::CpiAccounts,
    error::LightSdkError,
    instruction::{PackedAddressTreeInfo, ValidityProof},
};
use solana_program::{
    account_info::AccountInfo, program::invoke_signed, pubkey::Pubkey, rent::Rent,
    system_instruction, sysvar::Sysvar,
};

use crate::MyPdaAccount;

/// INITS a PDA and compresses it into a new compressed account.
pub fn create_dynamic_pda(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    let mut instruction_data = instruction_data;
    let instruction_data = CreateDynamicPdaInstructionData::deserialize(&mut instruction_data)
        .map_err(|e| {
            solana_program::msg!("Borsh deserialization error: {:?}", e);
            LightSdkError::ProgramError(e.into())
        })?;

    let fee_payer = &accounts[0];
    // UNCHECKED: ...caller program checks this.
    let solana_account = &accounts[1];
    let rent_recipient = &accounts[2];
    let config_account = &accounts[3];
    let system_program = &accounts[4];

    // Load config
    let config = CompressibleConfig::load_checked(config_account, &crate::ID)?;

    // CHECK: rent recipient from config
    if rent_recipient.key != &config.rent_recipient {
        solana_program::msg!(
            "rent recipient mismatch {:?} != {:?}",
            rent_recipient.key,
            config.rent_recipient
        );
        return Err(LightSdkError::ConstraintViolation);
    }

    // Derive PDA with seeds and bump
    // For this example, we'll use a simple seed pattern
    let seed_data = b"dynamic_pda"; // You can customize this based on your needs
    let (derived_pda, bump_seed) = Pubkey::find_program_address(&[seed_data], &crate::ID);

    // Verify the PDA matches what was passed in
    if derived_pda != *solana_account.key {
        solana_program::msg!(
            "PDA derivation mismatch. derived_pda: {:?} != solana_account.key: {:?}",
            derived_pda,
            solana_account.key
        );
        return Err(LightSdkError::ConstraintViolation);
    }

    // Calculate space needed for MyPdaAccount
    let account_space = std::mem::size_of::<MyPdaAccount>() + 8; // 8 bytes for discriminator

    // Calculate rent
    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(account_space);

    // Create the PDA account using system program
    let create_account_ix = system_instruction::create_account(
        fee_payer.key,
        solana_account.key,
        rent_lamports,
        account_space as u64,
        &crate::ID,
    );

    invoke_signed(
        &create_account_ix,
        &[
            fee_payer.clone(),
            solana_account.clone(),
            system_program.clone(),
        ],
        &[&[seed_data, &[bump_seed]]],
    )
    .map_err(|e| {
        solana_program::msg!("pda account create error: {:?}", e);
        LightSdkError::ProgramError(e)
    })?;

    // Initialize the PDA account data
    let mut pda_account_data = MyPdaAccount {
        compression_info: Some(CompressionInfo::new_decompressed()?),
        data: [1; 31], // Initialize with default data
    };

    // Serialize the initial data into the account - use scope to ensure borrow is dropped
    {
        let mut account_data = solana_account.data.borrow_mut();
        pda_account_data
            .serialize(&mut &mut account_data[..])
            .map_err(|e| {
                solana_program::msg!("pda account serialization error: {:?}", e);
                LightSdkError::ProgramError(e.into())
            })?;
    } // account_data borrow is dropped here

    // Cpi accounts
    let cpi_accounts_struct = CpiAccounts::new(fee_payer, &accounts[5..], crate::LIGHT_CPI_SIGNER);

    // the onchain PDA is the seed for the cPDA. this way devs don't have to
    // change their onchain PDA checks.
    let new_address_params = instruction_data
        .address_tree_info
        .into_new_address_params_packed(solana_account.key.to_bytes());

    solana_program::msg!("pda account data: {:?}", pda_account_data);

    // Use the efficient native variant that accepts pre-deserialized data
    compress_account_on_init_native::<MyPdaAccount>(
        &mut solana_account.clone(),
        &mut pda_account_data,
        &instruction_data.compressed_address,
        &new_address_params,
        instruction_data.output_state_tree_index,
        cpi_accounts_struct,
        &config.address_space,
        rent_recipient,
        instruction_data.proof,
    )?;

    Ok(())
}

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct CreateDynamicPdaInstructionData {
    pub proof: ValidityProof,
    pub compressed_address: [u8; 32],
    pub address_tree_info: PackedAddressTreeInfo,
    pub output_state_tree_index: u8,
}
