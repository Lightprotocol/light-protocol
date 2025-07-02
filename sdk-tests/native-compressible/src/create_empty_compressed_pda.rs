use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::{
    compressible::{compress_empty_account_on_init_native, CompressibleConfig, CompressionInfo},
    cpi::CpiAccountsSmall,
    error::LightSdkError,
    instruction::{PackedAddressTreeInfo, ValidityProof},
};
use solana_program::{
    account_info::AccountInfo, program::invoke_signed, pubkey::Pubkey, rent::Rent,
    system_instruction, sysvar::Sysvar,
};

use crate::MyPdaAccount;

/// INITS a PDA and creates an EMPTY compressed account without closing the PDA.
/// The PDA remains intact with its data, and an empty compressed account is created.
pub fn create_empty_compressed_pda(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    let mut instruction_data = instruction_data;
    let instruction_data = CreateEmptyCompressedPdaInstructionData::deserialize(
        &mut instruction_data,
    )
    .map_err(|e| {
        solana_program::msg!("Borsh deserialization error: {:?}", e);
        LightSdkError::ProgramError(e.into())
    })?;

    let fee_payer = &accounts[0];
    // UNCHECKED: ...caller program checks this.
    let solana_account = &accounts[1];
    let config_account = &accounts[2];
    let system_program = &accounts[3];

    // Load config
    let config = CompressibleConfig::load_checked(config_account, &crate::ID)?;

    // Derive PDA with seeds and bump
    // For this example, we'll use a simple seed pattern
    let seed_data = b"empty_compressed_pda"; // Different seed from regular dynamic PDA
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
        data: [1; 31], // Initialize with same data as regular PDA (for consistency)
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
    let cpi_accounts_struct =
        CpiAccountsSmall::new(fee_payer, &accounts[4..], crate::LIGHT_CPI_SIGNER);

    // the onchain PDA is the seed for the cPDA. this way devs don't have to
    // change their onchain PDA checks.
    let new_address_params = instruction_data
        .address_tree_info
        .into_new_address_params_packed(solana_account.key.to_bytes());

    solana_program::msg!("pda account data: {:?}", pda_account_data);
    solana_program::msg!("Creating EMPTY compressed account (PDA will remain intact)");

    // Use the new empty compression function - key difference from regular compression
    // Clone the account info to get mutability
    let mut solana_account_mut = solana_account.clone();
    compress_empty_account_on_init_native::<MyPdaAccount>(
        &mut solana_account_mut,
        &mut pda_account_data,
        &instruction_data.compressed_address,
        &new_address_params,
        instruction_data.output_state_tree_index,
        cpi_accounts_struct,
        &config.address_space,
        instruction_data.proof,
    )?;

    // Re-serialize the modified account data back to the on-chain account
    // This ensures compression_info changes persist
    {
        let mut account_data = solana_account.data.borrow_mut();
        pda_account_data
            .serialize(&mut &mut account_data[..])
            .map_err(|e| {
                solana_program::msg!("pda account re-serialization error: {:?}", e);
                LightSdkError::ProgramError(e.into())
            })?;
    }

    Ok(())
}

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct CreateEmptyCompressedPdaInstructionData {
    pub proof: ValidityProof,
    pub compressed_address: [u8; 32],
    pub address_tree_info: PackedAddressTreeInfo,
    pub output_state_tree_index: u8,
}
