//! Helper functions for preparing compressed accounts on init.

use light_compressed_account::{
    address::derive_address,
    instruction_data::{data::NewAddressParamsAssignedPacked, with_account_info::OutAccountInfo},
};
use light_hasher::errors::HasherError;
use solana_pubkey::Pubkey;

use crate::{compressed_account::CompressedAccountInfo, instruction::PackedAddressTreeInfo};

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
    // // Standard discriminator for PDA init TODO: restore after rebase
    // let discriminator = [255u8, 255u8, 255u8, 255u8, 255u8, 255u8, 255u8, 0u8];
    // // Data is always the PDA pubkey bytes
    // let data = pda_pubkey.to_bytes().to_vec();

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
    // let data_hash = Sha256BE::hash(&data)?;

    // Create and push CompressedAccountInfo
    account_infos.push(CompressedAccountInfo {
        address: Some(address),
        input: None,
        output: Some(OutAccountInfo {
            discriminator: [0u8; 8],
            output_merkle_tree_index: output_tree_index,
            lamports: 0,
            data: vec![],
            data_hash: [0u8; 32],
        }),
    });

    Ok(())
}
