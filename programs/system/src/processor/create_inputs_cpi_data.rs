use light_compressed_account::{
    hash_to_bn254_field_size_be,
    instruction_data::{
        insert_into_queues::{InsertIntoQueuesInstructionDataMut, InsertNullifierInput},
        traits::InstructionData,
    },
};
use light_hasher::{Hasher, Poseidon};
use light_program_profiler::profile;
use pinocchio::{account_info::AccountInfo, msg, program_error::ProgramError};

use crate::{
    accounts::remaining_account_checks::AcpAccount,
    context::{SystemContext, WrappedInstructionData},
    errors::SystemProgramError,
    Result,
};

/// Hashes the input compressed accounts and stores the results in the leaves array.
/// Merkle tree pubkeys are hashed and stored in the hashed_pubkeys array.
/// Merkle tree pubkeys should be ordered for efficiency.
#[inline(always)]
#[profile]
pub fn create_inputs_cpi_data<'a, 'info, T: InstructionData<'a>>(
    remaining_accounts: &'info [AccountInfo],
    instruction_data: &WrappedInstructionData<'a, T>,
    context: &mut SystemContext<'info>,
    cpi_ix_data: &mut InsertIntoQueuesInstructionDataMut<'_>,
    accounts: &[AcpAccount<'info>],
) -> Result<[u8; 32]> {
    if instruction_data.inputs_empty() {
        return Ok([0u8; 32]);
    }
    let mut owner_pubkey = instruction_data.owner();
    let mut hashed_owner = hash_to_bn254_field_size_be(&owner_pubkey.to_bytes());
    context
        .hashed_pubkeys
        .push((owner_pubkey.into(), hashed_owner));
    let mut current_hashed_mt = [0u8; 32];
    let mut hash_chain = [0u8; 32];

    let mut current_mt_index: u8 = 0;
    let mut is_first_iter = true;
    let mut seq_index = 0;
    let mut is_batched = true;
    for (j, input_compressed_account_with_context) in instruction_data.input_accounts().enumerate()
    {
        context
            .addresses
            .push(input_compressed_account_with_context.address());

        let merkle_context = input_compressed_account_with_context.merkle_context();
        #[allow(clippy::comparison_chain)]
        if current_mt_index != merkle_context.merkle_tree_pubkey_index || is_first_iter {
            is_first_iter = false;
            current_mt_index = merkle_context.merkle_tree_pubkey_index;
            current_hashed_mt = match &accounts
                .get(current_mt_index as usize)
                .ok_or(SystemProgramError::InputMerkleTreeIndexOutOfBounds)?
            {
                AcpAccount::BatchedStateTree(tree) => {
                    context.set_network_fee_v2(
                        tree.metadata.rollover_metadata.network_fee,
                        current_mt_index,
                    );
                    is_batched = true;
                    // We only set sequence number for batched input queues.
                    cpi_ix_data.insert_input_sequence_number(
                        &mut seq_index,
                        tree.pubkey(),
                        &tree.metadata.associated_queue,
                        tree.tree_type,
                        tree.queue_batches.next_index,
                    );
                    tree.hashed_pubkey
                }
                AcpAccount::StateTree(_) => {
                    is_batched = false;
                    let legacy_context = context
                        .get_legacy_merkle_context(current_mt_index)
                        .ok_or(SystemProgramError::MissingLegacyMerkleContext)?;
                    let network_fee = legacy_context.network_fee;
                    let hashed_pubkey = legacy_context.hashed_pubkey;
                    context.set_network_fee_v1(network_fee, current_mt_index)?;
                    hashed_pubkey
                }
                _ => {
                    msg!(format!("create_inputs_cpi_data {} ", current_mt_index).as_str());
                    return Err(
                        SystemProgramError::StateMerkleTreeAccountDiscriminatorMismatch.into(),
                    );
                }
            };
        }
        // Without cpi context all input compressed accounts have the same owner.
        // With cpi context the owners will be different.
        if owner_pubkey != *input_compressed_account_with_context.owner() {
            owner_pubkey = *input_compressed_account_with_context.owner();
            hashed_owner = context.get_or_hash_pubkey(owner_pubkey.into());
        }
        let merkle_context = input_compressed_account_with_context.merkle_context();
        let queue_index = context.get_index_or_insert(
            merkle_context.queue_pubkey_index,
            remaining_accounts,
            "Input queue (nullifier queue for V1 state trees, output queue for V2 state trees)",
        )?;
        let tree_index = context.get_index_or_insert(
            merkle_context.merkle_tree_pubkey_index,
            remaining_accounts,
            "Input tree",
        )?;

        cpi_ix_data.nullifiers[j] = InsertNullifierInput {
            account_hash: input_compressed_account_with_context
                .hash_with_hashed_values(
                    &hashed_owner,
                    &current_hashed_mt,
                    &merkle_context.leaf_index.into(),
                    is_batched,
                )
                .map_err(ProgramError::from)?,
            leaf_index: merkle_context.leaf_index,
            prove_by_index: merkle_context.prove_by_index() as u8,
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
    // TODO: benchmark the chaining.
    cpi_ix_data.num_queues = instruction_data
        .input_accounts()
        .enumerate()
        .filter(|(i, x)| {
            let candidate = x.merkle_context().queue_pubkey_index;
            !instruction_data
                .input_accounts()
                .take(*i)
                .any(|y| y.merkle_context().queue_pubkey_index == candidate)
        })
        .count() as u8;

    Ok(hash_chain)
}
