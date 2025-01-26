use std::mem;

use account_compression::{
    errors::AccountCompressionErrorCode, AddressMerkleTreeAccount, StateMerkleTreeAccount,
};
use anchor_lang::{prelude::*, Discriminator};
use light_batched_merkle_tree::{
    constants::{DEFAULT_BATCH_ADDRESS_TREE_HEIGHT, DEFAULT_BATCH_STATE_TREE_HEIGHT},
    merkle_tree::BatchedMerkleTreeAccount,
    queue::BatchedQueueAccount,
};
use light_concurrent_merkle_tree::zero_copy::ConcurrentMerkleTreeZeroCopy;
use light_hasher::{Discriminator as LightDiscriminator, Poseidon};
use light_indexed_merkle_tree::zero_copy::IndexedMerkleTreeZeroCopy;
use light_macros::heap_neutral;
use light_utils::{
    hash_to_bn254_field_size_be,
    hashchain::{create_hash_chain_from_slice, create_two_inputs_hash_chain},
};
use light_verifier::{
    select_verifying_key, verify_create_addresses_and_inclusion_proof,
    verify_create_addresses_proof, verify_inclusion_proof, CompressedProof,
};

use super::PackedReadOnlyAddress;
use crate::{
    errors::SystemProgramError,
    sdk::compressed_account::{
        PackedCompressedAccountWithMerkleContext, PackedReadOnlyCompressedAccount,
    },
    NewAddressParamsPacked,
};

const IS_READ_ONLY: bool = true;
const IS_NOT_READ_ONLY: bool = false;
const IS_STATE: bool = true;
const IS_NOT_STATE: bool = false;

#[inline(never)]
#[heap_neutral]
pub fn read_input_state_roots<'a>(
    remaining_accounts: &'a [AccountInfo<'_>],
    input_compressed_accounts_with_merkle_context: &'a [PackedCompressedAccountWithMerkleContext],
    read_only_accounts: &'a [PackedReadOnlyCompressedAccount],
    input_roots: &'a mut Vec<[u8; 32]>,
) -> Result<u8> {
    let mut state_tree_height = 0;
    for input_compressed_account_with_context in
        input_compressed_accounts_with_merkle_context.iter()
    {
        if input_compressed_account_with_context
            .merkle_context
            .queue_index
            .is_some()
        {
            continue;
        }
        let internal_height = read_root::<IS_NOT_READ_ONLY, IS_STATE>(
            &remaining_accounts[input_compressed_account_with_context
                .merkle_context
                .merkle_tree_pubkey_index as usize],
            input_compressed_account_with_context.root_index,
            input_roots,
        )?;
        if state_tree_height == 0 {
            state_tree_height = internal_height;
        } else if state_tree_height != internal_height {
            msg!(
                "tree height {} != internal height {}",
                state_tree_height,
                internal_height
            );
            return err!(SystemProgramError::InvalidStateTreeHeight);
        }
    }
    for readonly_input_account in read_only_accounts.iter() {
        if readonly_input_account.merkle_context.queue_index.is_some() {
            continue;
        }
        let internal_height = read_root::<IS_READ_ONLY, IS_STATE>(
            &remaining_accounts[readonly_input_account
                .merkle_context
                .merkle_tree_pubkey_index as usize],
            readonly_input_account.root_index,
            input_roots,
        )?;
        if state_tree_height == 0 {
            state_tree_height = internal_height;
        } else if state_tree_height != internal_height {
            msg!(
                "tree height {} != internal height {}",
                state_tree_height,
                internal_height
            );
            return err!(SystemProgramError::InvalidStateTreeHeight);
        }
    }
    Ok(state_tree_height)
}

#[inline(never)]
#[heap_neutral]
pub fn read_address_roots<'a>(
    remaining_accounts: &'a [AccountInfo<'_>],
    new_address_params: &'a [NewAddressParamsPacked],
    read_only_addresses: &'a [PackedReadOnlyAddress],
    address_roots: &'a mut Vec<[u8; 32]>,
) -> Result<u8> {
    let mut address_tree_height = 0;
    for new_address_param in new_address_params.iter() {
        let internal_height = read_root::<IS_NOT_READ_ONLY, IS_NOT_STATE>(
            &remaining_accounts[new_address_param.address_merkle_tree_account_index as usize],
            new_address_param.address_merkle_tree_root_index,
            address_roots,
        )?;
        if address_tree_height == 0 {
            address_tree_height = internal_height;
        } else if address_tree_height != internal_height {
            msg!(
                "tree height {} != internal height {}",
                address_tree_height,
                internal_height
            );
            return err!(SystemProgramError::InvalidAddressTreeHeight);
        }
    }
    for read_only_address in read_only_addresses.iter() {
        let internal_height = read_root::<IS_READ_ONLY, IS_NOT_STATE>(
            &remaining_accounts[read_only_address.address_merkle_tree_account_index as usize],
            read_only_address.address_merkle_tree_root_index,
            address_roots,
        )?;
        if address_tree_height == 0 {
            address_tree_height = internal_height;
        } else if address_tree_height != internal_height {
            msg!(
                "tree height {} != internal height {}",
                address_tree_height,
                internal_height
            );
            return err!(SystemProgramError::InvalidAddressTreeHeight);
        }
    }

    Ok(address_tree_height)
}

fn read_root<const IS_READ_ONLY: bool, const IS_STATE: bool>(
    merkle_tree_account_info: &AccountInfo<'_>,
    root_index: u16,
    roots: &mut Vec<[u8; 32]>,
) -> Result<u8> {
    let height;
    let mut discriminator_bytes = [0u8; 8];
    discriminator_bytes.copy_from_slice(&merkle_tree_account_info.try_borrow_data()?[0..8]);
    match discriminator_bytes {
        AddressMerkleTreeAccount::DISCRIMINATOR => {
            if IS_READ_ONLY {
                msg!("Read only addresses are only supported for batched address trees.");
                return err!(
                    AccountCompressionErrorCode::AddressMerkleTreeAccountDiscriminatorMismatch
                );
            }
            let merkle_tree = merkle_tree_account_info.try_borrow_data()?;
            let merkle_tree =
                IndexedMerkleTreeZeroCopy::<Poseidon, usize, 26, 16>::from_bytes_zero_copy(
                    &merkle_tree[8 + mem::size_of::<AddressMerkleTreeAccount>()..],
                )
                .map_err(ProgramError::from)?;
            height = merkle_tree.height as u8;
            (*roots).push(merkle_tree.roots[root_index as usize]);
        }
        BatchedMerkleTreeAccount::DISCRIMINATOR => {
            if IS_STATE {
                msg!("state_from_account_info");
                let merkle_tree =
                    BatchedMerkleTreeAccount::state_from_account_info(merkle_tree_account_info)
                        .map_err(ProgramError::from)?;
                (*roots).push(merkle_tree.root_history[root_index as usize]);
                height = merkle_tree.height as u8;
            } else {
                let merkle_tree =
                    BatchedMerkleTreeAccount::address_from_account_info(merkle_tree_account_info)
                        .map_err(ProgramError::from)?;
                height = merkle_tree.height as u8;
                (*roots).push(merkle_tree.root_history[root_index as usize]);
            }
        }
        StateMerkleTreeAccount::DISCRIMINATOR => {
            if IS_READ_ONLY {
                msg!("Read only addresses are only supported for batched address trees.");
                return err!(
                    AccountCompressionErrorCode::StateMerkleTreeAccountDiscriminatorMismatch
                );
            }

            let merkle_tree = &mut merkle_tree_account_info.try_borrow_mut_data()?;
            let merkle_tree = ConcurrentMerkleTreeZeroCopy::<Poseidon, 26>::from_bytes_zero_copy(
                &merkle_tree[8 + mem::size_of::<StateMerkleTreeAccount>()..],
            )
            .map_err(ProgramError::from)?;

            let fetched_roots = &merkle_tree.roots;

            (*roots).push(fetched_roots[root_index as usize]);
            height = merkle_tree.height as u8;
        }
        _ => {
            if IS_STATE {
                return err!(
                    AccountCompressionErrorCode::StateMerkleTreeAccountDiscriminatorMismatch
                );
            } else {
                return err!(
                    AccountCompressionErrorCode::AddressMerkleTreeAccountDiscriminatorMismatch
                );
            }
        }
    }
    Ok(height)
}

/// For each read-only account
/// 1. prove inclusion by index in the output queue if leaf index should exist in the output queue.
///    1.1. if inclusion was proven by index, return Ok.
/// 2. prove non-inclusion in the bloom filters
///    2.1. skip cleared batches.
///    2.2. prove non-inclusion in the bloom filters for each batch.
#[inline(always)]
pub fn verify_read_only_account_inclusion_by_index<'a>(
    remaining_accounts: &'a [AccountInfo<'_>],
    read_only_accounts: &'a [PackedReadOnlyCompressedAccount],
) -> Result<usize> {
    let mut num_prove_read_only_accounts_prove_by_index = 0;
    for read_only_account in read_only_accounts.iter() {
        let output_queue_account_info = &remaining_accounts[read_only_account
            .merkle_context
            .nullifier_queue_pubkey_index
            as usize];
        let output_queue =
            &mut BatchedQueueAccount::output_from_account_info(output_queue_account_info)
                .map_err(ProgramError::from)?;
        let merkle_tree_account_info =
            &remaining_accounts[read_only_account.merkle_context.merkle_tree_pubkey_index as usize];
        output_queue
            .check_is_associated(&(*merkle_tree_account_info.key).into())
            .map_err(ProgramError::from)?;

        // Checks inclusion by index in the output queue if leaf index should exist in the output queue.
        // Else does nothing.
        let proved_inclusion = output_queue
            .prove_inclusion_by_index(
                read_only_account.merkle_context.leaf_index as u64,
                &read_only_account.account_hash,
            )
            .map_err(|_| SystemProgramError::ReadOnlyAccountDoesNotExist)?;
        if read_only_account.merkle_context.queue_index.is_some() {
            num_prove_read_only_accounts_prove_by_index += 1;
        }
        // If a read-only account is marked as proven by index
        // inclusion proof by index has to be successful
        // -> proved_inclusion == true.
        if !proved_inclusion && read_only_account.merkle_context.queue_index.is_some() {
            msg!("Expected read-only account in the output queue but account does not exist.");
            return err!(SystemProgramError::ReadOnlyAccountDoesNotExist);
        }
        // If we prove inclusion by index we do not need to check non-inclusion in bloom filters.
        // Since proving inclusion by index of non-read
        // only accounts overwrites the leaf in the output queue.
        if !proved_inclusion {
            let merkle_tree =
                &mut BatchedMerkleTreeAccount::state_from_account_info(merkle_tree_account_info)
                    .map_err(ProgramError::from)?;
            merkle_tree
                .check_input_queue_non_inclusion(&read_only_account.account_hash)
                .map_err(|_| SystemProgramError::ReadOnlyAccountDoesNotExist)?;
        }
    }
    Ok(num_prove_read_only_accounts_prove_by_index)
}

#[inline(always)]
pub fn verify_read_only_address_queue_non_inclusion<'a>(
    remaining_accounts: &'a [AccountInfo<'_>],
    read_only_addresses: &'a [PackedReadOnlyAddress],
) -> Result<()> {
    if read_only_addresses.is_empty() {
        return Ok(());
    }
    let mut index = read_only_addresses[0].address_merkle_tree_account_index;
    let mut merkle_tree =
        BatchedMerkleTreeAccount::address_from_account_info(&remaining_accounts[index as usize])
            .map_err(ProgramError::from)?;
    for read_only_address in read_only_addresses.iter() {
        if index != read_only_address.address_merkle_tree_account_index {
            index = read_only_address.address_merkle_tree_account_index;
            merkle_tree = BatchedMerkleTreeAccount::address_from_account_info(
                &remaining_accounts[index as usize],
            )
            .map_err(ProgramError::from)?;
        }
        merkle_tree
            .check_input_queue_non_inclusion(&read_only_address.address)
            .map_err(|_| SystemProgramError::ReadOnlyAddressAlreadyExists)?;
    }
    Ok(())
}

/// Hashes the input compressed accounts and stores the results in the leaves array.
/// Merkle tree pubkeys are hashed and stored in the hashed_pubkeys array.
/// Merkle tree pubkeys should be ordered for efficiency.
#[inline(never)]
#[heap_neutral]
#[allow(unused_mut)]
pub fn hash_input_compressed_accounts<'a, 'b, 'c: 'info, 'info>(
    remaining_accounts: &'a [AccountInfo<'info>],
    input_compressed_accounts_with_merkle_context: &'a [PackedCompressedAccountWithMerkleContext],
    leaves: &'a mut Vec<[u8; 32]>,
    addresses: &'a mut [Option<[u8; 32]>],
    hashed_pubkeys: &'a mut Vec<(Pubkey, [u8; 32])>,
) -> Result<()> {
    let mut owner_pubkey = input_compressed_accounts_with_merkle_context[0]
        .compressed_account
        .owner;
    let mut hashed_owner = hash_to_bn254_field_size_be(&owner_pubkey.to_bytes())
        .unwrap()
        .0;
    hashed_pubkeys.push((owner_pubkey, hashed_owner));
    #[allow(unused)]
    let mut current_hashed_mt = [0u8; 32];

    let mut current_mt_index: i16 = -1;
    for (j, input_compressed_account_with_context) in input_compressed_accounts_with_merkle_context
        .iter()
        .enumerate()
    {
        // For heap neutrality we cannot allocate new heap memory in this function.
        if let Some(address) = &input_compressed_account_with_context
            .compressed_account
            .address
        {
            addresses[j] = Some(*address);
        }

        #[allow(clippy::comparison_chain)]
        if current_mt_index
            != input_compressed_account_with_context
                .merkle_context
                .merkle_tree_pubkey_index as i16
        {
            current_mt_index = input_compressed_account_with_context
                .merkle_context
                .merkle_tree_pubkey_index as i16;
            let merkle_tree_pubkey = remaining_accounts[input_compressed_account_with_context
                .merkle_context
                .merkle_tree_pubkey_index
                as usize]
                .key();
            current_hashed_mt = match hashed_pubkeys.iter().find(|x| x.0 == merkle_tree_pubkey) {
                Some(hashed_merkle_tree_pubkey) => hashed_merkle_tree_pubkey.1,
                None => {
                    let hashed_merkle_tree_pubkey =
                        hash_to_bn254_field_size_be(&merkle_tree_pubkey.to_bytes())
                            .unwrap()
                            .0;
                    hashed_pubkeys.push((merkle_tree_pubkey, hashed_merkle_tree_pubkey));
                    hashed_merkle_tree_pubkey
                }
            };
        }
        // Without cpi context all input compressed accounts have the same owner.
        // With cpi context the owners will be different.
        if owner_pubkey
            != input_compressed_account_with_context
                .compressed_account
                .owner
        {
            owner_pubkey = input_compressed_account_with_context
                .compressed_account
                .owner;
            hashed_owner = match hashed_pubkeys.iter().find(|x| {
                x.0 == input_compressed_account_with_context
                    .compressed_account
                    .owner
            }) {
                Some(hashed_owner) => hashed_owner.1,
                None => {
                    let hashed_owner = hash_to_bn254_field_size_be(
                        &input_compressed_account_with_context
                            .compressed_account
                            .owner
                            .to_bytes(),
                    )
                    .unwrap()
                    .0;
                    hashed_pubkeys.push((
                        input_compressed_account_with_context
                            .compressed_account
                            .owner,
                        hashed_owner,
                    ));
                    hashed_owner
                }
            };
        }
        leaves.push(
            input_compressed_account_with_context
                .compressed_account
                .hash_with_hashed_values::<Poseidon>(
                    &hashed_owner,
                    &current_hashed_mt,
                    &input_compressed_account_with_context
                        .merkle_context
                        .leaf_index,
                )?,
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[heap_neutral]
pub fn verify_proof(
    roots: &[[u8; 32]],
    leaves: &[[u8; 32]],
    address_roots: &[[u8; 32]],
    addresses: &[[u8; 32]],
    compressed_proof: &CompressedProof,
    address_tree_height: u8,
    state_tree_height: u8,
) -> anchor_lang::Result<()> {
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

        let vk = select_verifying_key(leaves.len(), addresses.len()).map_err(ProgramError::from)?;
        light_verifier::verify(&[public_input_hash], compressed_proof, vk)
            .map_err(ProgramError::from)?;
    } else if state_tree_height == 26 && address_tree_height == 26 {
        // legacy combined inclusion & non-inclusion proof
        verify_create_addresses_and_inclusion_proof(
            roots,
            leaves,
            address_roots,
            addresses,
            compressed_proof,
        )
        .map_err(ProgramError::from)?;
    } else if state_tree_height == 26 {
        // legacy inclusion proof
        verify_inclusion_proof(roots, leaves, compressed_proof).map_err(ProgramError::from)?;
    } else if address_tree_height == 26 {
        // legacy non-inclusion proof
        verify_create_addresses_proof(address_roots, addresses, compressed_proof)
            .map_err(ProgramError::from)?;
    } else {
        msg!("state tree height: {}", state_tree_height);
        msg!("address tree height: {}", address_tree_height);
        return err!(SystemProgramError::InvalidAddressTreeHeight);
    }

    Ok(())
}
