use account_compression::{context::AcpAccount, errors::AccountCompressionErrorCode};
use anchor_lang::prelude::*;
use light_hasher::{Hasher, Poseidon};
use light_utils::{
    hash_to_bn254_field_size_be,
    instruction::{
        insert_into_queues::{AppendNullifyCreateAddressInputs, InsertNullifierInput},
        instruction_data_zero_copy::ZPackedCompressedAccountWithMerkleContext,
    },
};

use crate::context::SystemContext;

/// Hashes the input compressed accounts and stores the results in the leaves array.
/// Merkle tree pubkeys are hashed and stored in the hashed_pubkeys array.
/// Merkle tree pubkeys should be ordered for efficiency.
#[inline(always)]
pub fn create_inputs_cpi_data<'a, 'b, 'c: 'info, 'info>(
    remaining_accounts: &'info [AccountInfo<'info>],
    input_compressed_accounts_with_merkle_context: &'a [ZPackedCompressedAccountWithMerkleContext<'a>],
    context: &mut SystemContext<'info>,
    cpi_ix_data: &mut AppendNullifyCreateAddressInputs<'a>,
    accounts: &[AcpAccount<'a, 'info>],
) -> Result<[u8; 32]> {
    if input_compressed_accounts_with_merkle_context.is_empty() {
        return Ok([0u8; 32]);
    }
    let mut owner_pubkey = input_compressed_accounts_with_merkle_context[0]
        .compressed_account
        .owner;
    let mut hashed_owner = hash_to_bn254_field_size_be(&owner_pubkey.to_bytes())
        .unwrap()
        .0;
    context
        .hashed_pubkeys
        .push((owner_pubkey.into(), hashed_owner));
    let mut current_hashed_mt = [0u8; 32];
    let mut hash_chain = [0u8; 32];

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
            context.addresses.push(Some(**address));
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
            current_hashed_mt = match &accounts[current_mt_index as usize] {
                AcpAccount::BatchedStateTree(tree) => {
                    context.set_network_fee(
                        tree.metadata.rollover_metadata.network_fee,
                        current_mt_index as u8,
                    );
                    tree.hashed_pubkey
                }
                AcpAccount::StateTree(_) => {
                    context
                        .get_legacy_merkle_context(current_mt_index as u8)
                        .unwrap()
                        .hashed_pubkey
                }
                _ => {
                    return Err(
                        AccountCompressionErrorCode::StateMerkleTreeAccountDiscriminatorMismatch
                            .into(),
                    );
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
            hashed_owner = context.get_or_hash_pubkey(owner_pubkey.into());
        }
        let queue_index = context.get_index_or_insert(
            input_compressed_account_with_context
                .merkle_context
                .nullifier_queue_pubkey_index,
            remaining_accounts,
        );
        let tree_index = context.get_index_or_insert(
            input_compressed_account_with_context
                .merkle_context
                .merkle_tree_pubkey_index,
            remaining_accounts,
        );
        cpi_ix_data.nullifiers[j] = InsertNullifierInput {
            account_hash: input_compressed_account_with_context
                .compressed_account
                .hash_with_hashed_values::<Poseidon>(
                    &hashed_owner,
                    &current_hashed_mt,
                    &input_compressed_account_with_context
                        .merkle_context
                        .leaf_index
                        .into(),
                )
                .map_err(ProgramError::from)?,
            leaf_index: input_compressed_account_with_context
                .merkle_context
                .leaf_index,
            prove_by_index: input_compressed_account_with_context
                .merkle_context
                .prove_by_index() as u8,
            queue_index,
            tree_index,
        };
        if j == 0 {
            hash_chain = cpi_ix_data.nullifiers[j].account_hash;
        } else {
            hash_chain = Poseidon::hashv(&[&hash_chain, &cpi_ix_data.nullifiers[j].account_hash])
                .map_err(ProgramError::from)?;
        }
    }
    cpi_ix_data.num_queues = input_compressed_accounts_with_merkle_context
        .iter()
        .enumerate()
        .filter(|(i, x)| {
            let candidate = x.merkle_context.nullifier_queue_pubkey_index;
            !input_compressed_accounts_with_merkle_context[..*i]
                .iter()
                .any(|y| y.merkle_context.nullifier_queue_pubkey_index == candidate)
        })
        .count() as u8;

    Ok(hash_chain)
}
