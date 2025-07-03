use crate::{
    account::LightAccount,
    address::PackedNewAddressParams,
    compressible::HasCompressionInfo,
    cpi::{CpiAccounts, CpiInputs},
    error::LightSdkError,
    instruction::ValidityProof,
    light_account_checks::AccountInfoTrait,
    LightDiscriminator,
};
#[cfg(feature = "anchor")]
use anchor_lang::{prelude::Account, AccountDeserialize, AccountSerialize, ToAccountInfo};
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize as BorshDeserialize, AnchorSerialize as BorshSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::instruction_data::data::ReadOnlyAddress;
use light_hasher::DataHasher;
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

pub const SOLANA_SYSTEM_PROGRAM_ID: Pubkey = Pubkey::new_from_array([1; 32]);

#[cfg(feature = "anchor")]
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
/// * `config` - The compression config to validate address spaces
/// * `read_only_addresses` - Optional read-only addresses for exclusion proofs
///
/// # Returns
/// * `Ok(())` if the PDA was compressed successfully
/// * `Err(LightSdkError)` if there was an error
pub fn compress_pda_new<'info, A>(
    pda_account: &mut Account<'info, A>,
    address: [u8; 32],
    new_address_params: PackedNewAddressParams,
    output_state_tree_index: u8,
    proof: ValidityProof,
    cpi_accounts: CpiAccounts<'_, 'info>,
    owner_program: &Pubkey,
    rent_recipient: &AccountInfo<'info>,
    address_space: &[Pubkey],
    read_only_addresses: Option<Vec<ReadOnlyAddress>>,
) -> Result<(), LightSdkError>
where
    A: DataHasher
        + LightDiscriminator
        + BorshSerialize
        + BorshDeserialize
        + Default
        + Clone
        + HasCompressionInfo
        + std::fmt::Debug,
    A: AccountSerialize + AccountDeserialize,
{
    compress_multiple_pdas_new::<A>(
        &mut [pda_account],
        &[address],
        &[new_address_params],
        &[output_state_tree_index],
        proof,
        cpi_accounts,
        owner_program,
        rent_recipient,
        address_space,
        read_only_addresses,
    )
}

#[cfg(feature = "anchor")]
/// Helper function to compress multiple onchain PDAs into new compressed
/// accounts.
///
/// This function handles the entire compression operation for multiple PDAs.
///
/// # Arguments
/// * `pda_accounts` - The PDA accounts to compress (will be closed)
/// * `addresses` - The addresses for the compressed accounts
/// * `new_address_params` - Address parameters for the compressed accounts
/// * `output_state_tree_indices` - Output state tree indices for the compressed
///   accounts
/// * `proof` - Single validity proof for all accounts
/// * `cpi_accounts` - Accounts needed for CPI
/// * `owner_program` - The program that will own the compressed accounts
/// * `rent_recipient` - The account to receive the PDAs' rent
/// * `address_space` - The address space (1-4 address_trees) to validate
///   uniqueness of addresses against.
/// * `read_only_addresses` - Optional read-only addresses for exclusion proofs
///
/// # Returns
/// * `Ok(())` if all PDAs were compressed successfully
/// * `Err(LightSdkError)` if there was an error
pub fn compress_multiple_pdas_new<'info, A>(
    pda_accounts: &mut [&mut Account<'info, A>],
    addresses: &[[u8; 32]],
    new_address_params: &[PackedNewAddressParams],
    output_state_tree_indices: &[u8],
    proof: ValidityProof,
    cpi_accounts: CpiAccounts<'_, 'info>,
    owner_program: &Pubkey,
    rent_recipient: &AccountInfo<'info>,
    address_space: &[Pubkey],
    read_only_addresses: Option<Vec<ReadOnlyAddress>>,
) -> Result<(), LightSdkError>
where
    A: DataHasher
        + LightDiscriminator
        + BorshSerialize
        + BorshDeserialize
        + Default
        + Clone
        + HasCompressionInfo
        + std::fmt::Debug,
    A: AccountSerialize + AccountDeserialize,
{
    if pda_accounts.len() != addresses.len()
        || pda_accounts.len() != new_address_params.len()
        || pda_accounts.len() != output_state_tree_indices.len()
    {
        return Err(LightSdkError::ConstraintViolation);
    }

    // TODO: consider hardcoding instead of checking manually.
    // TODO: consider more efficient way to check.
    // CHECK: primary address space matches config
    for params in new_address_params {
        let tree = cpi_accounts
            .get_tree_account_info(params.address_merkle_tree_account_index as usize)?
            .pubkey();
        if !address_space.iter().any(|a| a == &tree) {
            return Err(LightSdkError::ConstraintViolation);
        }
    }

    if let Some(ref addrs) = read_only_addresses {
        for ro in addrs {
            let ro_pubkey = Pubkey::new_from_array(ro.address_merkle_tree_pubkey.to_bytes());
            if !address_space.iter().any(|a| a == &ro_pubkey) {
                return Err(LightSdkError::ConstraintViolation);
            }
        }
    }

    let mut total_lamports = 0u64;
    let mut compressed_account_infos = Vec::new();

    // TODO: add support for Multiple PDA addresses!
    for (((pda_account, &address), &new_address_param), &output_state_tree_index) in pda_accounts
        .iter_mut()
        .zip(addresses.iter())
        .zip(new_address_params.iter())
        .zip(output_state_tree_indices.iter())
    {
        // Ensure the account is marked as compressed
        pda_account.compression_info_mut().set_compressed();

        // Create the compressed account with the PDA data
        let mut compressed_account =
            LightAccount::<'_, A>::new_init(owner_program, Some(address), output_state_tree_index);
        compressed_account.account = (***pda_account).clone();

        msg!("compressed_account: {:?}", compressed_account.account);
        compressed_account_infos.push(compressed_account.to_account_info()?);

        // Accumulate lamports
        total_lamports = total_lamports
            .checked_add(pda_account.to_account_info().lamports())
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }

    // Create CPI inputs with compressed accounts and new addresses
    let mut cpi_inputs =
        CpiInputs::new_with_address(proof, compressed_account_infos, new_address_params.to_vec());

    // Add read-only addresses if provided
    cpi_inputs.read_only_address = read_only_addresses;

    // Invoke light system program to create all compressed accounts
    cpi_inputs.invoke_light_system_program(cpi_accounts)?;

    // Close all PDA accounts
    // let dest_starting_lamports = rent_recipient.lamports();
    // **rent_recipient.try_borrow_mut_lamports()? = dest_starting_lamports
    //     .checked_add(total_lamports)
    //     .ok_or(ProgramError::ArithmeticOverflow)?;

    for pda_account in pda_accounts {
        // Decrement source account lamports

        use anchor_lang::AccountsClose;

        pda_account.close(rent_recipient.clone()).map_err(|err| {
            msg!("Error closing PDA account: {:?}", err);
            LightSdkError::ConstraintViolation
        })?;
    }

    Ok(())
}

/// Helper function to compress an onchain PDA into a new compressed account (native Solana).
///
/// This is the native Solana version that accepts pre-deserialized account data
/// to avoid double deserialization. Use this when you've already deserialized
/// the account data in your native Solana program.
///
/// # Arguments
/// * `pda_account_info` - The PDA account info (will be closed)
/// * `pda_account_data` - The pre-deserialized PDA account data
/// * `address` - The address for the compressed account
/// * `new_address_params` - Address parameters for the compressed account
/// * `output_state_tree_index` - Output state tree index for the compressed account
/// * `proof` - Validity proof
/// * `cpi_accounts` - Accounts needed for CPI
/// * `owner_program` - The program that will own the compressed account
/// * `rent_recipient` - The account to receive the PDA's rent
/// * `address_space` - The address space to validate uniqueness against
/// * `read_only_addresses` - Optional read-only addresses for exclusion proofs
///
/// # Returns
/// * `Ok(())` if the PDA was compressed successfully
/// * `Err(LightSdkError)` if there was an error
pub fn compress_pda_new_with_data<A>(
    pda_account_info: &AccountInfo,
    pda_account_data: &mut A,
    address: [u8; 32],
    new_address_params: PackedNewAddressParams,
    output_state_tree_index: u8,
    proof: ValidityProof,
    cpi_accounts: CpiAccounts,
    owner_program: &Pubkey,
    rent_recipient: &AccountInfo,
    address_space: &[Pubkey],
    read_only_addresses: Option<Vec<ReadOnlyAddress>>,
) -> Result<(), LightSdkError>
where
    A: DataHasher
        + LightDiscriminator
        + BorshSerialize
        + BorshDeserialize
        + Default
        + Clone
        + HasCompressionInfo
        + std::fmt::Debug,
{
    compress_multiple_pdas_new_with_data::<A>(
        &[pda_account_info],
        &mut [pda_account_data],
        &[address],
        &[new_address_params],
        &[output_state_tree_index],
        proof,
        cpi_accounts,
        owner_program,
        rent_recipient,
        address_space,
        read_only_addresses,
    )
}

/// Helper function to compress multiple onchain PDAs into new compressed accounts (native Solana).
///
/// This is the native Solana version that accepts pre-deserialized account data
/// to avoid double deserialization. Use this when you've already deserialized
/// the account data in your native Solana program.
///
/// # Arguments
/// * `pda_accounts_info` - The PDA account infos (will be closed)
/// * `pda_accounts_data` - The pre-deserialized PDA account data
/// * `addresses` - The addresses for the compressed accounts
/// * `new_address_params` - Address parameters for the compressed accounts
/// * `output_state_tree_indices` - Output state tree indices for the compressed accounts
/// * `proof` - Single validity proof for all accounts
/// * `cpi_accounts` - Accounts needed for CPI
/// * `owner_program` - The program that will own the compressed accounts
/// * `rent_recipient` - The account to receive the PDAs' rent
/// * `address_space` - The address space to validate uniqueness against
/// * `read_only_addresses` - Optional read-only addresses for exclusion proofs
///
/// # Returns
/// * `Ok(())` if all PDAs were compressed successfully
/// * `Err(LightSdkError)` if there was an error
pub fn compress_multiple_pdas_new_with_data<A>(
    pda_accounts_info: &[&AccountInfo],
    pda_accounts_data: &mut [&mut A],
    addresses: &[[u8; 32]],
    new_address_params: &[PackedNewAddressParams],
    output_state_tree_indices: &[u8],
    proof: ValidityProof,
    cpi_accounts: CpiAccounts,
    owner_program: &Pubkey,
    rent_recipient: &AccountInfo,
    address_space: &[Pubkey],
    read_only_addresses: Option<Vec<ReadOnlyAddress>>,
) -> Result<(), LightSdkError>
where
    A: DataHasher
        + LightDiscriminator
        + BorshSerialize
        + BorshDeserialize
        + Default
        + Clone
        + HasCompressionInfo
        + std::fmt::Debug,
{
    if pda_accounts_info.len() != pda_accounts_data.len()
        || pda_accounts_info.len() != addresses.len()
        || pda_accounts_info.len() != new_address_params.len()
        || pda_accounts_info.len() != output_state_tree_indices.len()
    {
        return Err(LightSdkError::ConstraintViolation);
    }

    // Address space validation
    for params in new_address_params {
        let tree = cpi_accounts
            .get_tree_account_info(params.address_merkle_tree_account_index as usize)?
            .pubkey();
        if !address_space.iter().any(|a| a == &tree) {
            return Err(LightSdkError::ConstraintViolation);
        }
    }

    if let Some(ref addrs) = read_only_addresses {
        for ro in addrs {
            let ro_pubkey = Pubkey::new_from_array(ro.address_merkle_tree_pubkey.to_bytes());
            if !address_space.iter().any(|a| a == &ro_pubkey) {
                return Err(LightSdkError::ConstraintViolation);
            }
        }
    }

    let mut total_lamports = 0u64;
    let mut compressed_account_infos = Vec::new();

    for (
        (((&pda_account_info, pda_data), &address), &_new_address_param),
        &output_state_tree_index,
    ) in pda_accounts_info
        .iter()
        .zip(pda_accounts_data.iter_mut())
        .zip(addresses.iter())
        .zip(new_address_params.iter())
        .zip(output_state_tree_indices.iter())
    {
        // Check that the PDA account is owned by the caller program
        if pda_account_info.owner != owner_program {
            msg!(
                "Invalid PDA owner. Expected: {}. Found: {}.",
                owner_program,
                pda_account_info.owner
            );
            return Err(LightSdkError::ConstraintViolation);
        }

        // Ensure the account is marked as compressed
        pda_data.compression_info_mut().set_compressed();

        // Create the compressed account with the PDA data
        let mut compressed_account =
            LightAccount::<'_, A>::new_init(owner_program, Some(address), output_state_tree_index);
        compressed_account.account = (*pda_data).clone();

        compressed_account_infos.push(compressed_account.to_account_info()?);

        // Accumulate lamports
        total_lamports = total_lamports
            .checked_add(pda_account_info.lamports())
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }

    // Create CPI inputs with compressed accounts and new addresses
    let mut cpi_inputs =
        CpiInputs::new_with_address(proof, compressed_account_infos, new_address_params.to_vec());

    // Add read-only addresses if provided
    cpi_inputs.read_only_address = read_only_addresses;

    // Invoke light system program to create all compressed accounts
    cpi_inputs.invoke_light_system_program(cpi_accounts)?;

    // Close all PDA accounts and serialize the modified data back
    let dest_starting_lamports = rent_recipient.lamports();
    **rent_recipient.try_borrow_mut_lamports()? = dest_starting_lamports
        .checked_add(total_lamports)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    for (pda_account_info, pda_data) in pda_accounts_info.iter().zip(pda_accounts_data.iter()) {
        // Serialize the modified data back to the account
        let mut account_data = pda_account_info.try_borrow_mut_data()?;
        pda_data
            .serialize(&mut &mut account_data[8..])
            .map_err(|_| LightSdkError::Borsh)?;

        // Decrement source account lamports
        **pda_account_info.try_borrow_mut_lamports()? = 0;
    }

    Ok(())
}
