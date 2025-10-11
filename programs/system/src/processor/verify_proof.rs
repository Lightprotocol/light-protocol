use light_batched_merkle_tree::constants::{
    DEFAULT_BATCH_ADDRESS_TREE_HEIGHT, DEFAULT_BATCH_STATE_TREE_HEIGHT,
};
use light_compressed_account::{
    hash_chain::{create_hash_chain_from_slice, create_two_inputs_hash_chain},
    instruction_data::{
        compressed_proof::CompressedProof,
        traits::{InputAccount, NewAddress},
        zero_copy::{ZPackedReadOnlyAddress, ZPackedReadOnlyCompressedAccount},
    },
};
use light_program_profiler::profile;
use light_verifier::{
    select_verifying_key, verify_create_addresses_and_inclusion_proof,
    verify_create_addresses_proof, verify_inclusion_proof,
};
use pinocchio::{msg, program_error::ProgramError};

use crate::{accounts::remaining_account_checks::AcpAccount, errors::SystemProgramError};

const IS_READ_ONLY: bool = true;
const IS_NOT_READ_ONLY: bool = false;
const IS_STATE: bool = true;
const IS_NOT_STATE: bool = false;

#[inline(always)]
#[profile]
pub fn read_input_state_roots<'a: 'b, 'b>(
    remaining_accounts: &[AcpAccount<'_>],
    input_compressed_accounts_with_merkle_context: impl Iterator<Item = &'b (dyn InputAccount<'a> + 'b)>,
    read_only_accounts: &[ZPackedReadOnlyCompressedAccount],
    input_roots: &mut Vec<[u8; 32]>,
) -> Result<u8, SystemProgramError> {
    let mut state_tree_height = 0;
    for input_compressed_account_with_context in input_compressed_accounts_with_merkle_context
        .filter(|account| !account.merkle_context().prove_by_index())
    {
        let internal_height = read_root::<IS_NOT_READ_ONLY, IS_STATE>(
            &remaining_accounts[input_compressed_account_with_context
                .merkle_context()
                .merkle_tree_pubkey_index as usize],
            input_compressed_account_with_context.root_index(),
            input_roots,
        )?;
        check_tree_height::<true>(&mut state_tree_height, internal_height)?;
    }
    for readonly_input_account in read_only_accounts
        .iter()
        .filter(|account| !account.merkle_context.prove_by_index())
    {
        let internal_height = read_root::<IS_READ_ONLY, IS_STATE>(
            &remaining_accounts[readonly_input_account
                .merkle_context
                .merkle_tree_pubkey_index as usize],
            readonly_input_account.root_index.into(),
            input_roots,
        )?;
        check_tree_height::<true>(&mut state_tree_height, internal_height)?;
    }
    Ok(state_tree_height)
}

/// Check that internal height matches tree height.
#[profile]
fn check_tree_height<const IS_STATE_TREE: bool>(
    tree_height: &mut u8,
    internal_height: u8,
) -> Result<(), SystemProgramError> {
    if *tree_height == 0 {
        *tree_height = internal_height;
    } else if *tree_height != internal_height {
        msg!(format!(
            "tree height {} != internal height {}",
            tree_height, internal_height
        )
        .as_str());
        if IS_STATE_TREE {
            return Err(SystemProgramError::InvalidStateTreeHeight);
        } else {
            return Err(SystemProgramError::InvalidAddressTreeHeight);
        }
    }
    Ok(())
}

#[inline(always)]
#[profile]
pub fn read_address_roots<'a, 'b: 'a>(
    remaining_accounts: &[AcpAccount<'_>],
    new_address_params: impl Iterator<Item = &'a (dyn NewAddress<'b> + 'a)>,
    read_only_addresses: &'a [ZPackedReadOnlyAddress],
    address_roots: &'a mut Vec<[u8; 32]>,
) -> Result<u8, SystemProgramError> {
    let mut address_tree_height = 0;
    for new_address_param in new_address_params {
        let internal_height = read_root::<IS_NOT_READ_ONLY, IS_NOT_STATE>(
            &remaining_accounts[new_address_param.address_merkle_tree_account_index() as usize],
            new_address_param.address_merkle_tree_root_index(),
            address_roots,
        )?;
        check_tree_height::<false>(&mut address_tree_height, internal_height)?;
    }
    for read_only_address in read_only_addresses.iter() {
        let internal_height = read_root::<IS_READ_ONLY, IS_NOT_STATE>(
            &remaining_accounts[read_only_address.address_merkle_tree_account_index as usize],
            read_only_address.address_merkle_tree_root_index.into(),
            address_roots,
        )?;
        check_tree_height::<false>(&mut address_tree_height, internal_height)?;
    }

    Ok(address_tree_height)
}

#[inline(always)]
#[profile]
fn read_root<const IS_READ_ONLY: bool, const IS_STATE: bool>(
    merkle_tree_account: &AcpAccount<'_>,
    root_index: u16,
    roots: &mut Vec<[u8; 32]>,
) -> Result<u8, SystemProgramError> {
    let height;
    match merkle_tree_account {
        AcpAccount::AddressTree((_, merkle_tree)) => {
            if IS_READ_ONLY {
                msg!("Read only addresses are only supported for batched address trees.");
                return Err(SystemProgramError::AddressMerkleTreeAccountDiscriminatorMismatch);
            }
            height = merkle_tree.height as u8;
            (*roots).push(merkle_tree.roots[root_index as usize]);
        }
        AcpAccount::BatchedStateTree(merkle_tree) => {
            (*roots).push(merkle_tree.root_history[root_index as usize]);
            height = merkle_tree.height as u8;
        }
        AcpAccount::BatchedAddressTree(merkle_tree) => {
            height = merkle_tree.height as u8;
            (*roots).push(merkle_tree.root_history[root_index as usize]);
        }
        AcpAccount::StateTree((_, merkle_tree)) => {
            if IS_READ_ONLY {
                msg!("Read only addresses are only supported for batched address trees.");
                return Err(SystemProgramError::StateMerkleTreeAccountDiscriminatorMismatch);
            }
            let fetched_roots = &merkle_tree.roots;

            (*roots).push(fetched_roots[root_index as usize]);
            height = merkle_tree.height as u8;
        }
        _ => {
            return if IS_STATE {
                Err(SystemProgramError::StateMerkleTreeAccountDiscriminatorMismatch)
            } else {
                Err(SystemProgramError::AddressMerkleTreeAccountDiscriminatorMismatch)
            }
        }
    }
    Ok(height)
}

#[allow(clippy::too_many_arguments)]
#[profile]
pub fn verify_proof(
    roots: &[[u8; 32]],
    leaves: &[[u8; 32]],
    address_roots: &[[u8; 32]],
    addresses: &[[u8; 32]],
    compressed_proof: &CompressedProof,
    address_tree_height: u8,
    state_tree_height: u8,
) -> crate::Result<()> {
    if state_tree_height as u32 == DEFAULT_BATCH_STATE_TREE_HEIGHT
        || address_tree_height as u32 == DEFAULT_BATCH_ADDRESS_TREE_HEIGHT
    {
        let public_input_hash = if !leaves.is_empty() && !addresses.is_empty() {
            // combined inclusion & non-inclusion proof
            let inclusion_hash =
                create_two_inputs_hash_chain(roots, leaves).map_err(ProgramError::from)?;
            let non_inclusion_hash = create_two_inputs_hash_chain(address_roots, addresses)
                .map_err(ProgramError::from)?;
            create_hash_chain_from_slice(&[inclusion_hash, non_inclusion_hash])
                .map_err(ProgramError::from)?
        } else if !leaves.is_empty() {
            // inclusion proof
            create_two_inputs_hash_chain(roots, leaves).map_err(ProgramError::from)?
        } else {
            // non-inclusion proof
            create_two_inputs_hash_chain(address_roots, addresses).map_err(ProgramError::from)?
        };

        let vk = select_verifying_key(leaves.len(), addresses.len())
            .map_err(|_| ProgramError::from(SystemProgramError::ProofVerificationFailed))?;
        light_verifier::verify(&[public_input_hash], compressed_proof, vk)
            .map_err(|_| ProgramError::from(SystemProgramError::ProofVerificationFailed))?;
    } else if state_tree_height == 26 && address_tree_height == 26 {
        // legacy combined inclusion & non-inclusion proof
        verify_create_addresses_and_inclusion_proof(
            roots,
            leaves,
            address_roots,
            addresses,
            compressed_proof,
        )
        .map_err(|_| ProgramError::from(SystemProgramError::ProofVerificationFailed))?;
    } else if state_tree_height == 26 {
        // legacy inclusion proof
        verify_inclusion_proof(roots, leaves, compressed_proof)
            .map_err(|_| ProgramError::from(SystemProgramError::ProofVerificationFailed))?;
    } else if address_tree_height == 26 {
        // legacy non-inclusion proof
        verify_create_addresses_proof(address_roots, addresses, compressed_proof)
            .map_err(|_| ProgramError::from(SystemProgramError::ProofVerificationFailed))?;
    } else {
        msg!(format!("state tree height: {}", state_tree_height).as_str());
        msg!(format!("address tree height: {}", address_tree_height).as_str());
        return Err(SystemProgramError::InvalidTreeHeight.into());
    }

    Ok(())
}
