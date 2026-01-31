//! Helper functions for preparing compressed accounts on init.

use light_compressed_account::{
    address::derive_address,
    instruction_data::{
        data::NewAddressParamsAssignedPacked,
        with_account_info::{CompressedAccountInfo, OutAccountInfo},
    },
};
use light_compressible::DECOMPRESSED_PDA_DISCRIMINATOR;
use light_hasher::{errors::HasherError, sha256::Sha256BE, Hasher};
use light_sdk_types::constants::RENT_SPONSOR_SEED;
use light_sdk_types::instruction::PackedAddressTreeInfo;
use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;
use solana_sysvar::{rent::Rent, Sysvar};

use crate::error::LightPdaError;
use light_account_checks::checks::check_mut;

/// Prepare a compressed account for a PDA during initialization.
///
/// This function handles the common pattern of:
/// 1. Deriving the compressed address from the PDA pubkey seed
/// 2. Creating NewAddressParamsAssignedPacked for the address tree
/// 3. Building CompressedAccountInfo with hashed PDA pubkey data
///
/// Uses:
/// - Discriminator: `[255, 255, 255, 255, 255, 255, 255, 0]` - marks this as a
///   rent-free PDA placeholder (distinct from actual account data discriminators)
/// - Data: PDA pubkey bytes (32 bytes) - allows lookup/verification of the
///   compressed account by its on-chain PDA address
///
/// # Arguments
/// * `pda_pubkey` - The PDA's pubkey (used as address seed and data)
/// * `address_tree_pubkey` - The address Merkle tree pubkey
/// * `address_tree_info` - Packed address tree info from CreateAccountsProof
/// * `output_tree_index` - Output state tree index
/// * `assigned_account_index` - Index in the accounts array (for assigned_account_index)
/// * `program_id` - The program ID (owner of the compressed account)
/// * `new_address_params` - Vector to push new address params into
/// * `account_infos` - Vector to push compressed account info into
#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub fn prepare_compressed_account_on_init(
    pda_pubkey: &Pubkey,
    address_tree_pubkey: &Pubkey,
    address_tree_info: &PackedAddressTreeInfo,
    output_tree_index: u8,
    assigned_account_index: u8,
    program_id: &Pubkey,
    new_address_params: &mut Vec<NewAddressParamsAssignedPacked>,
    account_infos: &mut Vec<CompressedAccountInfo>,
) -> Result<(), HasherError> {
    // Data is always the PDA pubkey bytes
    let data = pda_pubkey.to_bytes().to_vec();

    // Derive compressed address from PDA pubkey seed
    let address_seed = pda_pubkey.to_bytes();
    let address = derive_address(
        &address_seed,
        &address_tree_pubkey.to_bytes(),
        &program_id.to_bytes(),
    );

    // Create and push new address params
    new_address_params.push(NewAddressParamsAssignedPacked {
        seed: address_seed,
        address_merkle_tree_account_index: address_tree_info.address_merkle_tree_pubkey_index,
        address_queue_account_index: address_tree_info.address_queue_pubkey_index,
        address_merkle_tree_root_index: address_tree_info.root_index,
        assigned_to_account: true,
        assigned_account_index,
    });

    // Hash the data for the compressed account
    let data_hash = Sha256BE::hash(&data)?;

    // Create and push CompressedAccountInfo
    account_infos.push(CompressedAccountInfo {
        address: Some(address),
        input: None,
        output: Some(OutAccountInfo {
            discriminator: DECOMPRESSED_PDA_DISCRIMINATOR,
            output_merkle_tree_index: output_tree_index,
            lamports: 0,
            data,
            data_hash,
        }),
    });

    Ok(())
}

/// Safe variant that validates PDA derivation before preparing compressed account.
///
/// # Arguments
/// * `pda_pubkey` - The PDA's pubkey (used as address seed and data)
/// * `pda_seeds` - Seeds used to derive the PDA (without bump)
/// * `pda_bump` - The bump seed for the PDA
/// * `address_tree_pubkey` - The address Merkle tree pubkey
/// * `address_tree_info` - Packed address tree info from CreateAccountsProof
/// * `output_tree_index` - Output state tree index
/// * `assigned_account_index` - Index in the accounts array
/// * `program_id` - The program ID (owner of the compressed account)
/// * `new_address_params` - Vector to push new address params into
/// * `account_infos` - Vector to push compressed account info into
#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub fn prepare_compressed_account_on_init_checked(
    pda_pubkey: &Pubkey,
    pda_seeds: &[&[u8]],
    pda_bump: u8,
    address_tree_pubkey: &Pubkey,
    address_tree_info: &PackedAddressTreeInfo,
    output_tree_index: u8,
    assigned_account_index: u8,
    program_id: &Pubkey,
    new_address_params: &mut Vec<NewAddressParamsAssignedPacked>,
    account_infos: &mut Vec<CompressedAccountInfo>,
) -> Result<(), ProgramError> {
    // Validate PDA derivation
    let bump_slice = [pda_bump];
    let seeds_with_bump: Vec<&[u8]> = pda_seeds
        .iter()
        .copied()
        .chain(std::iter::once(bump_slice.as_slice()))
        .collect();

    let expected_pda = Pubkey::create_program_address(&seeds_with_bump, program_id)
        .map_err(|_| ProgramError::InvalidSeeds)?;

    if pda_pubkey != &expected_pda {
        solana_msg::msg!(
            "PDA key mismatch: expected {:?}, got {:?}",
            expected_pda,
            pda_pubkey
        );
        return Err(ProgramError::InvalidSeeds);
    }

    prepare_compressed_account_on_init(
        pda_pubkey,
        address_tree_pubkey,
        address_tree_info,
        output_tree_index,
        assigned_account_index,
        program_id,
        new_address_params,
        account_infos,
    )
    .map_err(|e| LightPdaError::from(e).into())
}

/// Reimburse the fee payer for rent paid during PDA initialization.
///
/// When using Anchor's `#[account(init)]` with `#[light_account(init)]`, the fee_payer
/// pays for rent-exemption. Since these become rent-free compressed accounts, this function
/// transfers the total rent amount back to the fee_payer from the program's rent sponsor PDA.
///
/// # Arguments
/// * `created_accounts` - Slice of AccountInfo for the PDAs that were created
/// * `fee_payer` - The account that paid for rent (will receive reimbursement)
/// * `rent_sponsor` - The program's rent sponsor PDA (must be mutable, pays reimbursement)
/// * `program_id` - The program ID (for deriving rent sponsor PDA bump)
///
/// # Seeds
/// The rent sponsor PDA is derived using: `[RENT_SPONSOR_SEED]`
pub fn reimburse_rent<'info>(
    created_accounts: &[AccountInfo<'info>],
    fee_payer: &AccountInfo<'info>,
    rent_sponsor: &AccountInfo<'info>,
    program_id: &Pubkey,
) -> Result<(), ProgramError> {
    if created_accounts.is_empty() {
        return Ok(());
    }

    // Calculate total rent-exemption for all created accounts
    let rent = Rent::get()?;
    let total_lamports: u64 = created_accounts
        .iter()
        .map(|acc| rent.minimum_balance(acc.data_len()))
        .sum();

    if total_lamports == 0 {
        return Ok(());
    }

    // Derive rent sponsor bump
    let (expected_rent_sponsor, rent_sponsor_bump) =
        Pubkey::find_program_address(&[RENT_SPONSOR_SEED], program_id);

    // Verify the rent sponsor account matches expected PDA
    if rent_sponsor.key != &expected_rent_sponsor {
        solana_msg::msg!(
            "rent_sponsor mismatch: expected {:?}, got {:?}",
            expected_rent_sponsor,
            rent_sponsor.key
        );
        return Err(LightPdaError::InvalidRentSponsor.into());
    }

    // Validate accounts are writable for transfer
    check_mut(rent_sponsor).map_err(ProgramError::from)?;
    check_mut(fee_payer).map_err(ProgramError::from)?;

    // Transfer from rent sponsor to fee payer
    let transfer_ix = solana_system_interface::instruction::transfer(
        rent_sponsor.key,
        fee_payer.key,
        total_lamports,
    );

    let bump_bytes = [rent_sponsor_bump];
    let rent_sponsor_seeds: &[&[u8]] = &[RENT_SPONSOR_SEED, &bump_bytes];

    solana_cpi::invoke_signed(
        &transfer_ix,
        &[rent_sponsor.clone(), fee_payer.clone()],
        &[rent_sponsor_seeds],
    )?;

    Ok(())
}
