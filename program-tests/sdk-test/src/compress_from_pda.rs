use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::{
    account::LightAccount,
    cpi::{CpiAccounts, CpiAccountsConfig, CpiInputs},
    error::LightSdkError,
    instruction::ValidityProof,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, msg, program_error::ProgramError, pubkey::Pubkey,
    sysvar::Sysvar,
};

use crate::{create_pda::MyCompressedAccount, decompress_to_pda::DecompressedPdaAccount};

/// Compresses a PDA back into a compressed account
/// Anyone can call this after the timeout period has elapsed
pub fn compress_from_pda(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    msg!("Compressing PDA back to compressed account");

    let mut instruction_data = instruction_data;
    let instruction_data = CompressFromPdaInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| LightSdkError::Borsh)?;

    // Get accounts
    let fee_payer = &accounts[0];
    let pda_account = &accounts[1];
    let rent_recipient = &accounts[2]; // Hardcoded by caller program
    let _system_program = &accounts[3];

    // Verify the PDA account is owned by our program
    if pda_account.owner != &crate::ID {
        msg!("PDA account not owned by this program");
        return Err(LightSdkError::ConstraintViolation);
    }

    // Read and deserialize PDA data
    let pda_data = pda_account.try_borrow_data()?;

    // Check discriminator
    if &pda_data[..8] != b"decomppd" {
        msg!("Invalid PDA discriminator");
        return Err(LightSdkError::ConstraintViolation);
    }

    let decompressed_pda = DecompressedPdaAccount::deserialize(&mut &pda_data[8..])
        .map_err(|_| LightSdkError::Borsh)?;

    // Check if enough time has passed
    let clock = Clock::get().map_err(|_| LightSdkError::Borsh)?;
    let current_slot = clock.slot;
    let slots_elapsed = current_slot.saturating_sub(decompressed_pda.last_written_slot);

    if slots_elapsed < decompressed_pda.slots_until_compression {
        msg!(
            "Cannot compress yet. {} slots remaining",
            decompressed_pda
                .slots_until_compression
                .saturating_sub(slots_elapsed)
        );
        return Err(LightSdkError::ConstraintViolation);
    }

    // Derive PDA to verify it matches
    let (pda_pubkey, _pda_bump) = Pubkey::find_program_address(
        &[
            b"decompressed_pda",
            &decompressed_pda.compressed_address,
            &instruction_data.additional_seed,
        ],
        &crate::ID,
    );

    if pda_pubkey != *pda_account.key {
        msg!("PDA derivation mismatch");
        return Err(LightSdkError::ConstraintViolation);
    }

    // Drop the borrow before we close the account
    drop(pda_data);

    // Close the PDA account and send rent to recipient
    let pda_lamports = pda_account.lamports();
    **pda_account.try_borrow_mut_lamports()? = 0;
    **rent_recipient.try_borrow_mut_lamports()? = rent_recipient
        .lamports()
        .checked_add(pda_lamports)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // Clear the PDA data
    pda_account.try_borrow_mut_data()?.fill(0);

    // Now create the compressed account with the latest data
    let mut compressed_account = LightAccount::<'_, MyCompressedAccount>::new_init(
        &crate::ID,
        Some(decompressed_pda.compressed_address),
        instruction_data.output_merkle_tree_index,
    );

    compressed_account.data = decompressed_pda.data;

    // Set up CPI accounts for light system program
    let mut config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
    config.sol_pool_pda = true; // We're compressing SOL

    let cpi_accounts = CpiAccounts::new_with_config(
        fee_payer,
        &accounts[instruction_data.system_accounts_offset as usize..],
        config,
    );

    // Create CPI inputs
    let mut cpi_inputs = CpiInputs::new_with_address(
        instruction_data.proof,
        vec![compressed_account.to_account_info()?],
        vec![instruction_data.new_address_params],
    );

    // Set compression parameters
    // We're compressing the lamports that were in the PDA
    cpi_inputs.compress_or_decompress_lamports = Some(instruction_data.lamports_to_compress);
    cpi_inputs.is_compress = true;

    // Invoke light system program
    cpi_inputs.invoke_light_system_program(cpi_accounts)?;

    msg!("Successfully compressed PDA back to compressed account");
    Ok(())
}

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct CompressFromPdaInstructionData {
    pub proof: ValidityProof,
    pub new_address_params: light_sdk::address::PackedNewAddressParams,
    pub output_merkle_tree_index: u8,
    pub additional_seed: [u8; 32], // Must match the seed used in decompression
    pub lamports_to_compress: u64,
    pub system_accounts_offset: u8,
}
