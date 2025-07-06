use crate::{
    account::LightAccount,
    address::{v1::derive_address, PackedNewAddressParams},
    cpi::{CpiAccounts, CpiInputs},
    error::LightSdkError,
    instruction::ValidityProof,
    light_account_checks::AccountInfoTrait,
    LightDiscriminator,
};
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize as BorshDeserialize, AnchorSerialize as BorshSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::DataHasher;
use solana_account_info::AccountInfo;
use solana_clock::Clock;
use solana_msg::msg;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;
use solana_sysvar::Sysvar;

use crate::compressible::compress_pda::PdaTimingData;

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

        let current_slot = Clock::get()?.slot;
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
        pda_account.assign(&Pubkey::default());
    }

    Ok(())
}
