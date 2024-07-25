use crate::{
    sdk::{accounts::InvokeAccounts, compressed_account::PackedCompressedAccountWithMerkleContext},
    NewAddressParamsPacked,
};
use account_compression::{
    utils::check_discrimininator::check_discriminator, AddressMerkleTreeAccount,
    StateMerkleTreeAccount,
};
use anchor_lang::{prelude::*, Bumps};
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

#[inline(never)]
#[heap_neutral]
pub fn fetch_input_compressed_account_roots<
    'a,
    'b,
    'c: 'info,
    'info,
    A: InvokeAccounts<'info> + Bumps,
>(
    input_compressed_accounts_with_merkle_context: &'a [PackedCompressedAccountWithMerkleContext],
    ctx: &'a Context<'a, 'b, 'c, 'info, A>,
    roots: &'a mut [[u8; 32]],
) -> Result<()> {
    for (i, input_compressed_account_with_context) in input_compressed_accounts_with_merkle_context
        .iter()
        .enumerate()
    {
        let merkle_tree = &ctx.remaining_accounts[input_compressed_account_with_context
            .merkle_context
            .merkle_tree_pubkey_index as usize];
        let merkle_tree = merkle_tree.try_borrow_data()?;
        check_discriminator::<StateMerkleTreeAccount>(&merkle_tree)?;
        let merkle_tree = ConcurrentMerkleTreeZeroCopy::<Poseidon, 26>::from_bytes_zero_copy(
            &merkle_tree[8 + mem::size_of::<StateMerkleTreeAccount>()..],
        )
        .map_err(ProgramError::from)?;
        let fetched_roots = &merkle_tree.roots;

        roots[i] = fetched_roots[input_compressed_account_with_context.root_index as usize];
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
    ctx: &'a Context<'a, 'b, 'c, 'info, A>,
    roots: &'a mut [[u8; 32]],
) -> Result<()> {
    for (i, new_address_param) in new_address_params.iter().enumerate() {
        let merkle_tree = ctx.remaining_accounts
            [new_address_param.address_merkle_tree_account_index as usize]
            .to_account_info();
        let merkle_tree = merkle_tree.try_borrow_data()?;
        check_discriminator::<AddressMerkleTreeAccount>(&merkle_tree)?;
        let merkle_tree =
            IndexedMerkleTreeZeroCopy::<Poseidon, usize, 26, 16>::from_bytes_zero_copy(
                &merkle_tree[8 + mem::size_of::<AddressMerkleTreeAccount>()..],
            )
            .map_err(ProgramError::from)?;
        let fetched_roots = &merkle_tree.roots;

        roots[i] = fetched_roots[new_address_param.address_merkle_tree_root_index as usize];
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
        if input_compressed_account_with_context
            .merkle_context
            .queue_index
            .is_some()
        {
            unimplemented!("Queue index is not supported.");
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

#[heap_neutral]
pub fn verify_state_proof(
    roots: &[[u8; 32]],
    leaves: &[[u8; 32]],
    address_roots: &[[u8; 32]],
    addresses: &[[u8; 32]],
    compressed_proof: &CompressedProof,
) -> anchor_lang::Result<()> {
    if !addresses.is_empty() && !leaves.is_empty() {
        verify_create_addresses_and_merkle_proof_zkp(
            roots,
            leaves,
            address_roots,
            addresses,
            compressed_proof,
        )
        .map_err(ProgramError::from)?;
    } else if !addresses.is_empty() {
        verify_create_addresses_zkp(address_roots, addresses, compressed_proof)
            .map_err(ProgramError::from)?;
    } else {
        verify_merkle_proof_zkp(roots, leaves, compressed_proof).map_err(ProgramError::from)?;
    }
    Ok(())
}
