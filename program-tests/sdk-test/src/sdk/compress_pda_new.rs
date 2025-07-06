use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::DataHasher;
use light_sdk::{
    account::LightAccount,
    address::{v1::derive_address, PackedNewAddressParams},
    cpi::{CpiAccounts, CpiInputs},
    error::LightSdkError,
    instruction::ValidityProof,
    light_account_checks::AccountInfoTrait,
    LightDiscriminator,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, msg, program_error::ProgramError, pubkey::Pubkey,
    sysvar::Sysvar,
};

use crate::sdk::compress_pda::PdaTimingData;

/// Helper function to compress an onchain PDA into a new compressed account.
///
/// This function handles the entire compression operation: creates a compressed account,
/// copies the PDA data, and closes the onchain PDA.
///
/// # Arguments
/// * `pda_account` - The PDA account to compress (will be closed)
/// * `address` - The address for the compressed account
/// * `new_address_params` - Address parameters for the compressed account
/// * `output_state_tree_index` - Output state tree index for the compressed account
/// * `proof` - Validity proof
/// * `cpi_accounts` - Accounts needed for CPI
/// * `owner_program` - The program that will own the compressed account
/// * `rent_recipient` - The account to receive the PDA's rent
/// * `expected_address_space` - Optional expected address space pubkey to validate against
///
/// # Returns
/// * `Ok(())` if the PDA was compressed successfully
/// * `Err(LightSdkError)` if there was an error
pub fn compress_pda_new<'info, A>(
    pda_account: &AccountInfo<'info>,
    address: [u8; 32],
    new_address_params: PackedNewAddressParams,
    output_state_tree_index: u8,
    proof: ValidityProof,
    cpi_accounts: CpiAccounts<'_, 'info>,
    owner_program: &Pubkey,
    rent_recipient: &AccountInfo<'info>,
    expected_address_space: &Pubkey,
) -> Result<(), LightSdkError>
where
    A: DataHasher
        + LightDiscriminator
        + BorshSerialize
        + BorshDeserialize
        + Default
        + PdaTimingData
        + Clone,
{
    compress_multiple_pdas_new::<A>(
        &[pda_account],
        &[address],
        vec![new_address_params],
        &[output_state_tree_index],
        proof,
        cpi_accounts,
        owner_program,
        rent_recipient,
        expected_address_space,
    )
}

/// Helper function to compress multiple onchain PDAs into new compressed accounts.
///
/// This function handles the entire compression operation for multiple PDAs.
///
/// # Arguments
/// * `pda_accounts` - The PDA accounts to compress (will be closed)
/// * `addresses` - The addresses for the compressed accounts
/// * `new_address_params` - Address parameters for the compressed accounts
/// * `output_state_tree_indices` - Output state tree indices for the compressed accounts
/// * `proof` - Single validity proof for all accounts
/// * `cpi_accounts` - Accounts needed for CPI
/// * `owner_program` - The program that will own the compressed accounts
/// * `rent_recipient` - The account to receive the PDAs' rent
/// * `expected_address_space` - Optional expected address space pubkey to validate against
///
/// # Returns
/// * `Ok(())` if all PDAs were compressed successfully
/// * `Err(LightSdkError)` if there was an error
pub fn compress_multiple_pdas_new<'info, A>(
    pda_accounts: &[&AccountInfo<'info>],
    addresses: &[[u8; 32]],
    new_address_params: Vec<PackedNewAddressParams>,
    output_state_tree_indices: &[u8],
    proof: ValidityProof,
    cpi_accounts: CpiAccounts<'_, 'info>,
    owner_program: &Pubkey,
    rent_recipient: &AccountInfo<'info>,
    expected_address_space: &Pubkey,
) -> Result<(), LightSdkError>
where
    A: DataHasher
        + LightDiscriminator
        + BorshSerialize
        + BorshDeserialize
        + Default
        + PdaTimingData
        + Clone,
{
    if pda_accounts.len() != addresses.len()
        || pda_accounts.len() != new_address_params.len()
        || pda_accounts.len() != output_state_tree_indices.len()
    {
        return Err(LightSdkError::ConstraintViolation);
    }

    // CHECK: address space.
    for params in &new_address_params {
        let address_tree_account = cpi_accounts
            .get_tree_account_info(params.address_merkle_tree_account_index as usize)?;
        if address_tree_account.pubkey() != *expected_address_space {
            msg!(
                "Invalid address space. Expected: {}. Found: {}.",
                expected_address_space,
                address_tree_account.pubkey()
            );
            return Err(LightSdkError::ConstraintViolation);
        }
    }

    let current_slot = Clock::get()?.slot;
    let mut total_lamports = 0u64;
    let mut compressed_account_infos = Vec::new();

    for ((pda_account, &address), &output_state_tree_index) in pda_accounts
        .iter()
        .zip(addresses.iter())
        .zip(output_state_tree_indices.iter())
    {
        // Check that the PDA account is owned by the caller program
        if pda_account.owner != owner_program {
            msg!(
                "Invalid PDA owner for {}. Expected: {}. Found: {}.",
                pda_account.key,
                owner_program,
                pda_account.owner
            );
            return Err(LightSdkError::ConstraintViolation);
        }

        // Deserialize the PDA data to check timing fields
        let pda_data = pda_account.try_borrow_data()?;
        let pda_account_data =
            A::try_from_slice(&pda_data[8..]).map_err(|_| LightSdkError::Borsh)?;
        drop(pda_data);

        let last_written_slot = pda_account_data.last_written_slot();
        let slots_until_compression = pda_account_data.slots_until_compression();

        if current_slot < last_written_slot + slots_until_compression {
            msg!(
                "Cannot compress {} yet. {} slots remaining",
                pda_account.key,
                (last_written_slot + slots_until_compression).saturating_sub(current_slot)
            );
            return Err(LightSdkError::ConstraintViolation);
        }

        // Create the compressed account with the PDA data
        let mut compressed_account =
            LightAccount::<'_, A>::new_init(owner_program, Some(address), output_state_tree_index);
        compressed_account.account = pda_account_data;

        compressed_account_infos.push(compressed_account.to_account_info()?);

        // Accumulate lamports
        total_lamports = total_lamports
            .checked_add(pda_account.lamports())
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }

    // Create CPI inputs with all compressed accounts and new addresses
    let cpi_inputs =
        CpiInputs::new_with_address(proof, compressed_account_infos, new_address_params);

    // Invoke light system program to create all compressed accounts
    cpi_inputs.invoke_light_system_program(cpi_accounts)?;

    // Close all PDA accounts
    let dest_starting_lamports = rent_recipient.lamports();
    **rent_recipient.try_borrow_mut_lamports()? = dest_starting_lamports
        .checked_add(total_lamports)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    for pda_account in pda_accounts {
        // Decrement source account lamports
        **pda_account.try_borrow_mut_lamports()? = 0;
        // Clear all account data
        pda_account.try_borrow_mut_data()?.fill(0);
        // Assign ownership back to the system program
        pda_account.assign(&solana_program::system_program::ID);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decompress_dynamic_pda::MyPdaAccount;
    use light_sdk::cpi::CpiAccountsConfig;
    use light_sdk::instruction::PackedAddressTreeInfo;

    /// Test instruction that demonstrates compressing an onchain PDA into a new compressed account
    pub fn test_compress_pda_new(
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> Result<(), LightSdkError> {
        msg!("Testing compress PDA into new compressed account");

        #[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
        struct TestInstructionData {
            pub proof: ValidityProof,
            pub address_tree_info: PackedAddressTreeInfo,
            pub output_state_tree_index: u8,
            pub system_accounts_offset: u8,
        }

        let mut instruction_data = instruction_data;
        let instruction_data = TestInstructionData::deserialize(&mut instruction_data)
            .map_err(|_| LightSdkError::Borsh)?;

        // Get accounts
        let fee_payer = &accounts[0];
        let pda_account = &accounts[1];
        let rent_recipient = &accounts[2];

        // Set up CPI accounts
        let cpi_accounts = CpiAccounts::new_with_config(
            fee_payer,
            &accounts[instruction_data.system_accounts_offset as usize..],
            CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER),
        );

        // Get the address tree pubkey
        let address_tree_pubkey = instruction_data
            .address_tree_info
            .get_tree_pubkey(&cpi_accounts)?;

        // This can happen offchain too!
        let (address, address_seed) = derive_address(
            &[pda_account.key.as_ref()],
            &address_tree_pubkey,
            &crate::ID,
        );

        // Create new address params
        let new_address_params = instruction_data
            .address_tree_info
            .into_new_address_params_packed(address_seed);

        // Compress the PDA - this handles everything internally
        compress_pda_new::<MyPdaAccount>(
            pda_account,
            address,
            new_address_params,
            instruction_data.output_state_tree_index,
            instruction_data.proof,
            cpi_accounts,
            &crate::ID,
            rent_recipient,
            &crate::create_dynamic_pda::ADDRESS_SPACE,
        )?;

        msg!("PDA compressed successfully into new compressed account");
        Ok(())
    }
}
