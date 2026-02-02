//! Helper functions for preparing compressed accounts on init.

use alloc::vec::Vec;

use crate::instruction::PackedAddressTreeInfo;
use light_account_checks::AccountInfoTrait;
use light_compressed_account::{
    address::derive_address,
    instruction_data::{
        data::NewAddressParamsAssignedPacked,
        with_account_info::{CompressedAccountInfo, OutAccountInfo},
    },
};
use light_compressible::DECOMPRESSED_PDA_DISCRIMINATOR;
use light_hasher::{errors::HasherError, sha256::Sha256BE, Hasher};

use crate::error::LightSdkTypesError;

/// Prepare a compressed account for a PDA during initialization.
///
/// This function handles the common pattern of:
/// 1. Deriving the compressed address from the PDA pubkey seed
/// 2. Creating NewAddressParamsAssignedPacked for the address tree
/// 3. Building CompressedAccountInfo with hashed PDA pubkey data
///
/// Uses `[u8; 32]` for all pubkey parameters - framework-agnostic.
#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub fn prepare_compressed_account_on_init(
    pda_pubkey: &[u8; 32],
    address_tree_pubkey: &[u8; 32],
    address_tree_info: &PackedAddressTreeInfo,
    output_tree_index: u8,
    assigned_account_index: u8,
    program_id: &[u8; 32],
    new_address_params: &mut Vec<NewAddressParamsAssignedPacked>,
    account_infos: &mut Vec<CompressedAccountInfo>,
) -> Result<(), HasherError> {
    // Data is always the PDA pubkey bytes
    let data = pda_pubkey.to_vec();

    // Derive compressed address from PDA pubkey seed
    let address = derive_address(pda_pubkey, address_tree_pubkey, program_id);

    // Create and push new address params
    new_address_params.push(NewAddressParamsAssignedPacked {
        seed: *pda_pubkey,
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

/// Reimburse the fee_payer for rent paid during PDA creation.
///
/// During Anchor `init`, the fee_payer pays rent for PDA accounts.
/// This function transfers the total rent amount from the program-owned
/// rent_sponsor PDA back to the fee_payer.
///
/// Uses direct lamport manipulation (no CPI) since rent_sponsor is owned
/// by the calling program.
pub fn reimburse_rent<AI: AccountInfoTrait>(
    created_accounts: &[AI],
    fee_payer: &AI,
    rent_sponsor: &AI,
    _program_id: &[u8; 32],
) -> Result<(), LightSdkTypesError> {
    let mut total_rent: u64 = 0;
    for account in created_accounts {
        total_rent = total_rent
            .checked_add(account.lamports())
            .ok_or(LightSdkTypesError::ConstraintViolation)?;
    }

    if total_rent > 0 {
        rent_sponsor
            .transfer_lamports(fee_payer, total_rent)
            .map_err(LightSdkTypesError::AccountError)?;
    }

    Ok(())
}
