use crate::{
    errors::SystemProgramError,
    sdk::{
        accounts::InvokeAccounts,
        compressed_account::{
            FetchRoot, PackedCompressedAccountWithMerkleContext, PackedReadOnlyCompressedAccount,
        },
    },
    NewAddressParamsPacked,
};
use account_compression::{
    batched_merkle_tree::{BatchedMerkleTreeAccount, ZeroCopyBatchedMerkleTreeAccount},
    batched_queue::ZeroCopyBatchedQueueAccount,
    errors::AccountCompressionErrorCode,
    AddressMerkleTreeAccount, StateMerkleTreeAccount,
};
use anchor_lang::{prelude::*, Bumps, Discriminator};
use light_concurrent_merkle_tree::zero_copy::ConcurrentMerkleTreeZeroCopy;
use light_hasher::Poseidon;
use light_indexed_merkle_tree::zero_copy::IndexedMerkleTreeZeroCopy;
use light_macros::heap_neutral;
use light_utils::hash_to_bn254_field_size_be;
use light_verifier::{
    verify_create_addresses_and_merkle_proof_zkp, verify_create_addresses_zkp,
    verify_merkle_proof_zkp, CompressedProof,
};
use std::mem;

use super::PackedReadOnlyAddress;

#[inline(never)]
#[heap_neutral]
pub fn fetch_input_compressed_account_roots<
    'a,
    'b,
    'c: 'info,
    'info,
    A: InvokeAccounts<'info> + Bumps,
    F: FetchRoot,
>(
    input_compressed_accounts_with_merkle_context: &'a [F],
    ctx: &'a Context<'a, 'b, 'c, 'info, A>,
    roots: &'a mut Vec<[u8; 32]>,
) -> Result<()> {
    for input_compressed_account_with_context in
        input_compressed_accounts_with_merkle_context.iter()
    {
        // Skip accounts which prove inclusion by index in output queue.
        if input_compressed_account_with_context
            .get_merkle_context()
            .queue_index
            .is_some()
        {
            continue;
        }
        let merkle_tree_account_info = &ctx.remaining_accounts[input_compressed_account_with_context
            .get_merkle_context()
            .merkle_tree_pubkey_index
            as usize];
        let mut discriminator_bytes = [0u8; 8];
        discriminator_bytes.copy_from_slice(&merkle_tree_account_info.try_borrow_data()?[0..8]);
        match discriminator_bytes {
            StateMerkleTreeAccount::DISCRIMINATOR => {
                let merkle_tree = &mut merkle_tree_account_info.try_borrow_mut_data()?;
                let merkle_tree =
                    ConcurrentMerkleTreeZeroCopy::<Poseidon, 26>::from_bytes_zero_copy(
                        &merkle_tree[8 + mem::size_of::<StateMerkleTreeAccount>()..],
                    )
                    .map_err(ProgramError::from)?;
                let fetched_roots = &merkle_tree.roots;

                (*roots).push(
                    fetched_roots[input_compressed_account_with_context.get_root_index() as usize],
                );
            }
            BatchedMerkleTreeAccount::DISCRIMINATOR => {
                let merkle_tree =
                    ZeroCopyBatchedMerkleTreeAccount::state_tree_from_account_info_mut(
                        merkle_tree_account_info,
                    )
                    .map_err(ProgramError::from)?;
                (*roots).push(
                    merkle_tree.root_history
                        [input_compressed_account_with_context.get_root_index() as usize],
                );
            }
            _ => {
                return err!(
                    AccountCompressionErrorCode::StateMerkleTreeAccountDiscriminatorMismatch
                );
            }
        }
    }
    Ok(())
}

#[inline(never)]
#[heap_neutral]
pub fn fetch_roots_address_merkle_tree<
    'a,
    'b,
    'c: 'info,
    'info,
    A: InvokeAccounts<'info> + Bumps,
>(
    new_address_params: &'a [NewAddressParamsPacked],
    read_only_addresses: &'a [PackedReadOnlyAddress],
    ctx: &'a Context<'a, 'b, 'c, 'info, A>,
    roots: &'a mut Vec<[u8; 32]>,
) -> Result<()> {
    for new_address_param in new_address_params.iter() {
        fetch_address_root::<false, A>(
            ctx,
            new_address_param.address_merkle_tree_account_index,
            new_address_param.address_merkle_tree_root_index,
            roots,
        )?;
    }
    for read_only_address in read_only_addresses.iter() {
        fetch_address_root::<true, A>(
            ctx,
            read_only_address.address_merkle_tree_account_index,
            read_only_address.address_merkle_tree_root_index,
            roots,
        )?;
    }
    Ok(())
}

/// For each input account which is marked to be proven by index
/// 1. check that it can exist in the output queue
/// - note the output queue checks whether the value acutally exists in the queue
/// - the purpose of this check is to catch marked input accounts which shouldn't be proven by index
#[inline(always)]
pub fn verify_input_accounts_proof_by_index<'a>(
    remaining_accounts: &'a [AccountInfo<'_>],
    input_accounts: &'a [PackedCompressedAccountWithMerkleContext],
) -> Result<()> {
    for account in input_accounts.iter() {
        if account.merkle_context.queue_index.is_some() {
            let output_queue_account_info =
                &remaining_accounts[account.merkle_context.nullifier_queue_pubkey_index as usize];
            let output_queue =
                &mut ZeroCopyBatchedQueueAccount::output_queue_from_account_info_mut(
                    output_queue_account_info,
                )
                .map_err(ProgramError::from)?;
            output_queue.could_exist_in_batches(account.merkle_context.leaf_index as u64)?;
        }
    }
    Ok(())
}

/// For each read-only account
/// 1. prove inclusion by index in the output queue if leaf index should exist in the output queue.
/// 1.1. if proved inclusion by index, return Ok.
/// 2. prove non-inclusion in the bloom filters
/// 2.1. skip wiped batches.
/// 2.2. prove non-inclusion in the bloom filters for each batch.
#[inline(always)]
pub fn verify_read_only_account_inclusion<'a>(
    remaining_accounts: &'a [AccountInfo<'_>],
    read_only_accounts: &'a [PackedReadOnlyCompressedAccount],
) -> Result<()> {
    for read_only_account in read_only_accounts.iter() {
        let output_queue_account_info = &remaining_accounts[read_only_account
            .merkle_context
            .nullifier_queue_pubkey_index
            as usize];
        let output_queue = &mut ZeroCopyBatchedQueueAccount::output_queue_from_account_info_mut(
            output_queue_account_info,
        )
        .map_err(ProgramError::from)?;
        let proved_inclusion = output_queue
            .prove_inclusion_by_index(
                read_only_account.merkle_context.leaf_index as u64,
                &read_only_account.account_hash,
            )
            .map_err(|_| SystemProgramError::ReadOnlyAccountDoesNotExist)?;
        if !proved_inclusion && read_only_account.merkle_context.queue_index.is_some() {
            msg!("Expected read-only account in the output queue but does not exist.");
            return err!(SystemProgramError::ReadOnlyAccountDoesNotExist);
        }
        // If we prove inclusion by index we do not need to check non-inclusion in bloom filters.
        if !proved_inclusion {
            let merkle_tree_account_info = &remaining_accounts
                [read_only_account.merkle_context.merkle_tree_pubkey_index as usize];
            let merkle_tree =
                &mut ZeroCopyBatchedMerkleTreeAccount::state_tree_from_account_info_mut(
                    merkle_tree_account_info,
                )
                .map_err(ProgramError::from)?;

            let num_bloom_filters = merkle_tree.bloom_filter_stores.len();
            for i in 0..num_bloom_filters {
                let bloom_filter_store = merkle_tree.bloom_filter_stores[i].as_mut_slice();
                let batch = &merkle_tree.batches[i];
                if !batch.bloom_filter_is_wiped {
                    batch
                        .check_non_inclusion(&read_only_account.account_hash, bloom_filter_store)
                        .map_err(|_| SystemProgramError::ReadOnlyAccountDoesNotExist)?;
                }
            }
        }
    }
    Ok(())
}

#[inline(always)]
pub fn verify_read_only_address_queue_non_inclusion<'a>(
    remaining_accounts: &'a [AccountInfo<'_>],
    read_only_addresses: &'a [PackedReadOnlyAddress],
) -> Result<()> {
    for read_only_address in read_only_addresses.iter() {
        let merkle_tree_account_info =
            &remaining_accounts[read_only_address.address_merkle_tree_account_index as usize];
        let merkle_tree =
            &mut ZeroCopyBatchedMerkleTreeAccount::address_tree_from_account_info_mut(
                merkle_tree_account_info,
            )
            .map_err(ProgramError::from)?;

        let num_bloom_filters = merkle_tree.bloom_filter_stores.len();
        for i in 0..num_bloom_filters {
            let bloom_filter_store = merkle_tree.bloom_filter_stores[i].as_mut_slice();
            let batch = &merkle_tree.batches[i];
            match batch.check_non_inclusion(&read_only_address.address, bloom_filter_store) {
                Ok(_) => {}
                Err(_) => {
                    return err!(SystemProgramError::ReadOnlyAddressAlreadyExists);
                }
            }
        }
    }
    Ok(())
}

fn fetch_address_root<
    'a,
    'b,
    'c: 'info,
    'info,
    const IS_READ_ONLY: bool,
    A: InvokeAccounts<'info> + Bumps,
>(
    ctx: &'a Context<'a, 'b, 'c, 'info, A>,
    address_merkle_tree_account_index: u8,
    address_merkle_tree_root_index: u16,
    roots: &'a mut Vec<[u8; 32]>,
) -> Result<()> {
    let merkle_tree_account_info =
        &ctx.remaining_accounts[address_merkle_tree_account_index as usize];
    let mut discriminator_bytes = [0u8; 8];
    discriminator_bytes.copy_from_slice(&merkle_tree_account_info.try_borrow_data()?[0..8]);
    match discriminator_bytes {
        AddressMerkleTreeAccount::DISCRIMINATOR => {
            if IS_READ_ONLY {
                msg!("Read only addresses are only supported for batch address trees.");
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
            (*roots).push(merkle_tree.roots[address_merkle_tree_root_index as usize]);
        }
        BatchedMerkleTreeAccount::DISCRIMINATOR => {
            let merkle_tree = ZeroCopyBatchedMerkleTreeAccount::address_tree_from_account_info_mut(
                merkle_tree_account_info,
            )
            .map_err(ProgramError::from)?;
            (*roots).push(merkle_tree.root_history[address_merkle_tree_root_index as usize]);
        }
        _ => {
            return err!(
                AccountCompressionErrorCode::AddressMerkleTreeAccountDiscriminatorMismatch
            );
        }
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
    leaves: &'a mut [[u8; 32]],
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
        match &input_compressed_account_with_context
            .compressed_account
            .address
        {
            Some(address) => addresses[j] = Some(*address),
            None => {}
        };

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
        leaves[j] = input_compressed_account_with_context
            .compressed_account
            .hash_with_hashed_values::<Poseidon>(
                &hashed_owner,
                &current_hashed_mt,
                &input_compressed_account_with_context
                    .merkle_context
                    .leaf_index,
            )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[heap_neutral]
pub fn verify_state_proof(
    input_compressed_accounts_with_merkle_context: &[PackedCompressedAccountWithMerkleContext],
    mut roots: Vec<[u8; 32]>,
    leaves: &[[u8; 32]],
    address_roots: &[[u8; 32]],
    addresses: &[[u8; 32]],
    read_only_accounts: &[PackedReadOnlyCompressedAccount],
    read_only_roots: &[[u8; 32]],
    compressed_proof: &CompressedProof,
) -> anchor_lang::Result<()> {
    // Filter out leaves that are not in the proof (proven by index).
    let mut proof_input_leaves = leaves
        .iter()
        .enumerate()
        .filter(|(x, _)| {
            input_compressed_accounts_with_merkle_context[*x]
                .merkle_context
                .queue_index
                .is_none()
        })
        .map(|x| *x.1)
        .collect::<Vec<[u8; 32]>>();

    read_only_accounts.iter().for_each(|x| {
        if x.merkle_context.queue_index.is_none() {
            proof_input_leaves.extend_from_slice(&[x.account_hash]);
        }
    });
    roots.extend_from_slice(read_only_roots);

    if !addresses.is_empty() && !proof_input_leaves.is_empty() {
        verify_create_addresses_and_merkle_proof_zkp(
            &roots,
            &proof_input_leaves,
            address_roots,
            addresses,
            compressed_proof,
        )
        .map_err(ProgramError::from)?;
    } else if !addresses.is_empty() {
        verify_create_addresses_zkp(address_roots, addresses, compressed_proof)
            .map_err(ProgramError::from)?;
    } else {
        verify_merkle_proof_zkp(&roots, &proof_input_leaves, compressed_proof)
            .map_err(ProgramError::from)?;
    }
    Ok(())
}

pub fn create_tx_hash(
    input_compressed_account_hashes: &[[u8; 32]],
    output_compressed_account_hashes: &[[u8; 32]],
    current_slot: u64,
) -> [u8; 32] {
    use light_hasher::Hasher;
    let mut tx_hash = input_compressed_account_hashes[0];
    for hash in input_compressed_account_hashes.iter().skip(1) {
        tx_hash = Poseidon::hashv(&[&tx_hash, hash]).unwrap();
    }
    tx_hash = Poseidon::hashv(&[&tx_hash, &current_slot.to_be_bytes()]).unwrap();
    for hash in output_compressed_account_hashes.iter() {
        tx_hash = Poseidon::hashv(&[&tx_hash, hash]).unwrap();
    }
    tx_hash
}
