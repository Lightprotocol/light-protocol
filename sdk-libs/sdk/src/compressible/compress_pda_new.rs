#[cfg(feature = "anchor")]
use anchor_lang::AccountsClose;
#[cfg(feature = "anchor")]
use anchor_lang::{prelude::Account, AccountDeserialize, AccountSerialize};
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize as BorshDeserialize, AnchorSerialize as BorshSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::DataHasher;
use solana_account_info::AccountInfo;
use solana_pubkey::Pubkey;

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
/// Wrapper to process a single onchain PDA for compression into a new compressed account.
/// Calls `process_accounts_for_compression_on_init` with single-element slices and invokes the CPI.
#[allow(clippy::too_many_arguments)]
pub fn compress_account_on_init<'info, A>(
    pda_account: &mut Account<'info, A>,
    address: &[u8; 32],
    new_address_param: &PackedNewAddressParams,
    output_state_tree_index: u8,
    cpi_accounts: CpiAccounts<'_, 'info>,
    owner_program: &Pubkey,
    address_space: &[Pubkey],
    rent_recipient: &AccountInfo<'info>,
    proof: ValidityProof,
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
    let mut pda_accounts: [&mut Account<'info, A>; 1] = [pda_account];
    let addresses: [[u8; 32]; 1] = [*address];
    let new_address_params: [PackedNewAddressParams; 1] = [*new_address_param];
    let output_state_tree_indices: [u8; 1] = [output_state_tree_index];

    let compressed_infos = prepare_accounts_for_compression_on_init(
        &mut pda_accounts,
        &addresses,
        &new_address_params,
        &output_state_tree_indices,
        &cpi_accounts,
        owner_program,
        address_space,
        rent_recipient,
    )?;

    // Create CPI inputs with all compressed accounts and new addresses
    let cpi_inputs = CpiInputs::new_with_address(proof, compressed_infos, vec![*new_address_param]);

    // Invoke light system program to create all compressed accounts
    cpi_inputs.invoke_light_system_program(cpi_accounts)?;

    Ok(())
}

#[cfg(feature = "anchor")]
/// Helper function to process multiple onchain PDAs for compression into new compressed accounts.
///
/// This function processes accounts of a single type and returns CompressedAccountInfo for CPI batching.
/// It allows the caller to handle the CPI invocation separately, enabling batching of multiple
/// different account types.
///
/// # Arguments
/// * `pda_accounts` - The PDA accounts to compress
/// * `addresses` - The addresses for the compressed accounts
/// * `new_address_params` - Address parameters for the compressed accounts
/// * `output_state_tree_indices` - Output state tree indices for the compressed accounts
/// * `cpi_accounts` - Accounts needed for validation
/// * `owner_program` - The program that will own the compressed accounts
/// * `address_space` - The address space to validate uniqueness against
///
/// # Returns
/// * `Ok(Vec<CompressedAccountInfo>)` - CompressedAccountInfo for CPI batching
/// * `Err(LightSdkError)` if there was an error
#[allow(clippy::too_many_arguments)]
pub fn prepare_accounts_for_compression_on_init<'info, A>(
    pda_accounts: &mut [&mut Account<'info, A>],
    addresses: &[[u8; 32]],
    new_address_params: &[PackedNewAddressParams],
    output_state_tree_indices: &[u8],
    cpi_accounts: &CpiAccounts<'_, 'info>,
    owner_program: &Pubkey,
    address_space: &[Pubkey],
    rent_recipient: &AccountInfo<'info>,
) -> Result<
    Vec<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>,
    LightSdkError,
>
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

    // Address space validation
    for params in new_address_params {
        let tree = cpi_accounts
            .get_tree_account_info(params.address_merkle_tree_account_index as usize)?
            .pubkey();
        if !address_space.iter().any(|a| a == &tree) {
            return Err(LightSdkError::ConstraintViolation);
        }
    }

    let mut compressed_account_infos = Vec::new();

    for (((pda_account, &address), &_new_address_param), &output_state_tree_index) in pda_accounts
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

        compressed_account_infos.push(compressed_account.to_account_info()?);

        // Close both PDA accounts
        pda_account
            .close(rent_recipient.clone())
            .map_err(|e| LightSdkError::ProgramError(e.into()))?;
    }

    Ok(compressed_account_infos)
}

// TODO: move.
// /// Helper function to compress multiple onchain PDAs into new compressed accounts (native Solana).
// ///
// /// This is the native Solana version that accepts pre-deserialized account data
// /// to avoid double deserialization. Use this when you've already deserialized
// /// the account data in your native Solana program.
// ///
// /// # Arguments
// /// * `pda_accounts_info` - The PDA account infos (will be closed)
// /// * `pda_accounts_data` - The pre-deserialized PDA account data
// /// * `addresses` - The addresses for the compressed accounts
// /// * `new_address_params` - Address parameters for the compressed accounts
// /// * `output_state_tree_indices` - Output state tree indices for the compressed accounts
// /// * `proof` - Single validity proof for all accounts
// /// * `cpi_accounts` - Accounts needed for CPI
// /// * `owner_program` - The program that will own the compressed accounts
// /// * `rent_recipient` - The account to receive the PDAs' rent
// /// * `address_space` - The address space to validate uniqueness against
// /// * `read_only_addresses` - Optional read-only addresses for exclusion proofs
// ///
// /// # Returns
// /// * `Ok(())` if all PDAs were compressed successfully
// /// * `Err(LightSdkError)` if there was an error
// pub fn compress_multiple_pdas_new_with_data<A>(
//     pda_accounts_info: &[&AccountInfo],
//     pda_accounts_data: &mut [&mut A],
//     addresses: &[[u8; 32]],
//     new_address_params: &[PackedNewAddressParams],
//     output_state_tree_indices: &[u8],
//     proof: ValidityProof,
//     cpi_accounts: CpiAccounts,
//     owner_program: &Pubkey,
//     rent_recipient: &AccountInfo,
//     address_space: &[Pubkey],
//     read_only_addresses: Option<Vec<ReadOnlyAddress>>,
// ) -> Result<(), LightSdkError>
// where
//     A: DataHasher
//         + LightDiscriminator
//         + BorshSerialize
//         + BorshDeserialize
//         + Default
//         + Clone
//         + HasCompressionInfo
//         + std::fmt::Debug,
// {
//     if pda_accounts_info.len() != pda_accounts_data.len()
//         || pda_accounts_info.len() != addresses.len()
//         || pda_accounts_info.len() != new_address_params.len()
//         || pda_accounts_info.len() != output_state_tree_indices.len()
//     {
//         return Err(LightSdkError::ConstraintViolation);
//     }

//     // Address space validation
//     for params in new_address_params {
//         let tree = cpi_accounts
//             .get_tree_account_info(params.address_merkle_tree_account_index as usize)?
//             .pubkey();
//         if !address_space.iter().any(|a| a == &tree) {
//             return Err(LightSdkError::ConstraintViolation);
//         }
//     }

//     if let Some(ref addrs) = read_only_addresses {
//         for ro in addrs {
//             let ro_pubkey = Pubkey::new_from_array(ro.address_merkle_tree_pubkey.to_bytes());
//             if !address_space.iter().any(|a| a == &ro_pubkey) {
//                 return Err(LightSdkError::ConstraintViolation);
//             }
//         }
//     }

//     let mut total_lamports = 0u64;
//     let mut compressed_account_infos = Vec::new();

//     for (
//         (((&pda_account_info, pda_data), &address), &_new_address_param),
//         &output_state_tree_index,
//     ) in pda_accounts_info
//         .iter()
//         .zip(pda_accounts_data.iter_mut())
//         .zip(addresses.iter())
//         .zip(new_address_params.iter())
//         .zip(output_state_tree_indices.iter())
//     {
//         // Check that the PDA account is owned by the caller program
//         if pda_account_info.owner != owner_program {
//             msg!(
//                 "Invalid PDA owner. Expected: {}. Found: {}.",
//                 owner_program,
//                 pda_account_info.owner
//             );
//             return Err(LightSdkError::ConstraintViolation);
//         }

//         // Ensure the account is marked as compressed
//         pda_data.compression_info_mut().set_compressed();

//         // Create the compressed account with the PDA data
//         let mut compressed_account =
//             LightAccount::<'_, A>::new_init(owner_program, Some(address), output_state_tree_index);
//         compressed_account.account = (*pda_data).clone();

//         compressed_account_infos.push(compressed_account.to_account_info()?);

//         // Accumulate lamports
//         total_lamports = total_lamports
//             .checked_add(pda_account_info.lamports())
//             .ok_or(ProgramError::ArithmeticOverflow)?;
//     }

//     // Create CPI inputs with compressed accounts and new addresses
//     let mut cpi_inputs =
//         CpiInputs::new_with_address(proof, compressed_account_infos, new_address_params.to_vec());

//     // Add read-only addresses if provided
//     cpi_inputs.read_only_address = read_only_addresses;

//     // Invoke light system program to create all compressed accounts
//     cpi_inputs.invoke_light_system_program(cpi_accounts)?;

//     // Close all PDA accounts and serialize the modified data back
//     let dest_starting_lamports = rent_recipient.lamports();
//     **rent_recipient.try_borrow_mut_lamports()? = dest_starting_lamports
//         .checked_add(total_lamports)
//         .ok_or(ProgramError::ArithmeticOverflow)?;

//     for (pda_account_info, pda_data) in pda_accounts_info.iter().zip(pda_accounts_data.iter()) {
//         // Serialize the modified data back to the account
//         let mut account_data = pda_account_info.try_borrow_mut_data()?;
//         pda_data
//             .serialize(&mut &mut account_data[8..])
//             .map_err(|_| LightSdkError::Borsh)?;

//         // Decrement source account lamports
//         **pda_account_info.try_borrow_mut_lamports()? = 0;
//     }

//     Ok(())
// }
