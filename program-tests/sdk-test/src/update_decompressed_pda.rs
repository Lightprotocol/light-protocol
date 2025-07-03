use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::error::LightSdkError;
use solana_program::{
    account_info::AccountInfo, clock::Clock, msg, pubkey::Pubkey, sysvar::Sysvar,
};

use crate::decompress_to_pda::DecompressedPdaAccount;

/// Updates the data in a decompressed PDA
/// This also updates the last_written_slot to the current slot
pub fn update_decompressed_pda(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    msg!("Updating decompressed PDA data");

    let mut instruction_data = instruction_data;
    let instruction_data = UpdateDecompressedPdaInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| LightSdkError::Borsh)?;

    // Get accounts
    let authority = &accounts[0]; // Must be a signer
    let pda_account = &accounts[1];

    // Verify authority is signer
    if !authority.is_signer {
        msg!("Authority must be a signer");
        return Err(LightSdkError::ConstraintViolation);
    }

    // Verify the PDA account is owned by our program
    if pda_account.owner != &crate::ID {
        msg!("PDA account not owned by this program");
        return Err(LightSdkError::ConstraintViolation);
    }

    // Read and deserialize PDA data
    let mut pda_data = pda_account.try_borrow_mut_data()?;

    // Check discriminator
    if &pda_data[..8] != b"decomppd" {
        msg!("Invalid PDA discriminator");
        return Err(LightSdkError::ConstraintViolation);
    }

    let mut decompressed_pda = DecompressedPdaAccount::deserialize(&mut &pda_data[8..])
        .map_err(|_| LightSdkError::Borsh)?;

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

    // Update the data
    decompressed_pda.data = instruction_data.new_data;

    // Update the last_written_slot to current slot
    let clock = Clock::get().map_err(|_| LightSdkError::Borsh)?;
    decompressed_pda.last_written_slot = clock.slot;

    // Write updated data back
    decompressed_pda
        .serialize(&mut &mut pda_data[8..])
        .map_err(|_| LightSdkError::Borsh)?;

    msg!("Successfully updated decompressed PDA data");
    Ok(())
}

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct UpdateDecompressedPdaInstructionData {
    pub new_data: [u8; 31],
    pub additional_seed: [u8; 32], // Must match the seed used in decompression
}
