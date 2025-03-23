use account_compression::{context::AcpAccount, errors::AccountCompressionErrorCode};
use anchor_lang::prelude::*;
use light_compressed_account::{
    hash_to_bn254_field_size_be,
    instruction_data::{
        insert_into_queues::{InsertIntoQueuesInstructionDataMut, MerkleTreeSequenceNumber},
        zero_copy::ZOutputCompressedAccountWithPackedContext,
    },
};
use light_hasher::{Hasher, Poseidon};

use crate::{context::SystemContext, errors::SystemProgramError};

/// Creates CPI accounts, instruction data, and performs checks.
/// - Merkle tree indices must be in order.
/// - Hashes output accounts for insertion and event.
/// - Collects sequence numbers for event.
///
/// Checks:
/// 1. Checks whether a Merkle tree is program owned, if so checks write
///    eligibility.
/// 2. Checks ordering of Merkle tree indices.
/// 3. Checks that addresses in output compressed accounts have been created or
///    exist in input compressed accounts. An address may not be used in an
///    output compressed accounts. This will close the account.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub fn create_outputs_cpi_data<'a, 'info>(
    output_compressed_accounts: &[ZOutputCompressedAccountWithPackedContext<'a>],
    remaining_accounts: &'info [AccountInfo<'info>],
    context: &mut SystemContext<'info>,
    cpi_ix_data: &mut InsertIntoQueuesInstructionDataMut<'a>,
    accounts: &[AcpAccount<'a, 'info>],
) -> Result<[u8; 32]> {
    if output_compressed_accounts.is_empty() {
        return Ok([0u8; 32]);
    }
    let mut current_index: i16 = -1;
    let mut num_leaves_in_tree: u32 = 0;
    let mut mt_next_index: u32 = 0;
    let mut hashed_merkle_tree = [0u8; 32];
    cpi_ix_data.start_output_appends = context.account_indices.len() as u8;
    let mut index_merkle_tree_account_account = cpi_ix_data.start_output_appends;
    let mut index_merkle_tree_account = 0;
    let number_of_merkle_trees =
        output_compressed_accounts.last().unwrap().merkle_tree_index as usize + 1;
    let mut merkle_tree_pubkeys =
        Vec::<light_compressed_account::pubkey::Pubkey>::with_capacity(number_of_merkle_trees);
    let mut hash_chain = [0u8; 32];
    let mut rollover_fee = 0;
    let mut is_batched = true;

    for (j, account) in output_compressed_accounts.iter().enumerate() {
        // if mt index == current index Merkle tree account info has already been added.
        // if mt index != current index, Merkle tree account info is new, add it.
        #[allow(clippy::comparison_chain)]
        if account.merkle_tree_index as i16 == current_index {
            // Do nothing, but it is the most common case.
        } else if account.merkle_tree_index as i16 > current_index {
            current_index = account.merkle_tree_index.into();

            let pubkey = match &accounts[current_index as usize] {
                AcpAccount::OutputQueue(output_queue) => {
                    context.set_network_fee(
                        output_queue.metadata.rollover_metadata.network_fee,
                        current_index as u8,
                    );
                    hashed_merkle_tree = output_queue.hashed_merkle_tree_pubkey;
                    rollover_fee = output_queue.metadata.rollover_metadata.rollover_fee;
                    mt_next_index = output_queue.batch_metadata.next_index as u32;
                    cpi_ix_data.output_sequence_numbers[index_merkle_tree_account as usize] =
                        MerkleTreeSequenceNumber {
                            pubkey: *output_queue.pubkey(),
                            seq: output_queue.batch_metadata.next_index.into(),
                        };
                    is_batched = true;
                    *output_queue.pubkey()
                }
                AcpAccount::StateTree((pubkey, tree)) => {
                    cpi_ix_data.output_sequence_numbers[index_merkle_tree_account as usize] =
                        MerkleTreeSequenceNumber {
                            pubkey: (*pubkey).into(),
                            seq: (tree.sequence_number() as u64 + 1).into(),
                        };
                    hashed_merkle_tree = context
                        .get_legacy_merkle_context(current_index as u8)
                        .unwrap()
                        .hashed_pubkey;
                    rollover_fee = context
                        .get_legacy_merkle_context(current_index as u8)
                        .unwrap()
                        .rollover_fee;
                    mt_next_index = tree.next_index() as u32;
                    is_batched = false;
                    (*pubkey).into()
                }
                _ => {
                    return err!(
                        AccountCompressionErrorCode::StateMerkleTreeAccountDiscriminatorMismatch
                    );
                }
            };
            // check Merkle tree uniqueness
            if merkle_tree_pubkeys.contains(&pubkey) {
                return err!(SystemProgramError::OutputMerkleTreeNotUnique);
            } else {
                merkle_tree_pubkeys.push(pubkey);
            }

            context.get_index_or_insert(account.merkle_tree_index, remaining_accounts);
            num_leaves_in_tree = 0;
            index_merkle_tree_account += 1;
            index_merkle_tree_account_account += 1;
        } else {
            // Check 2.
            // Output Merkle tree indices must be in order since we use the
            // number of leaves in a Merkle tree to determine the correct leaf
            // index. Since the leaf index is part of the hash this is security
            // critical.
            return err!(SystemProgramError::OutputMerkleTreeIndicesNotInOrder);
        }

        // Check 3.
        if let Some(address) = account.compressed_account.address {
            if let Some(position) = context
                .addresses
                .iter()
                .filter(|x| x.is_some())
                .position(|&x| x.unwrap() == *address)
            {
                context.addresses.remove(position);
            } else {
                msg!("Address {:?}, is no new address and does not exist in input compressed accounts.", address);
                msg!(
                    "Remaining compressed_account_addresses: {:?}",
                    context.addresses
                );
                return Err(SystemProgramError::InvalidAddress.into());
            }
        }

        cpi_ix_data.output_leaf_indices[j] = (mt_next_index + num_leaves_in_tree).into();
        num_leaves_in_tree += 1;
        if account.compressed_account.data.is_some() && context.invoking_program_id.is_none() {
            msg!("Invoking program is not provided.");
            msg!("Only program owned compressed accounts can have data.");
            return err!(SystemProgramError::InvokingProgramNotProvided);
        }
        let hashed_owner = match context
            .hashed_pubkeys
            .iter()
            .find(|x| x.0 == account.compressed_account.owner.into())
        {
            Some(hashed_owner) => hashed_owner.1,
            None => {
                let hashed_owner =
                    hash_to_bn254_field_size_be(&account.compressed_account.owner.to_bytes());
                context
                    .hashed_pubkeys
                    .push((account.compressed_account.owner.into(), hashed_owner));
                hashed_owner
            }
        };
        // Compute output compressed account hash.
        cpi_ix_data.leaves[j].leaf = account
            .compressed_account
            .hash_with_hashed_values(
                &hashed_owner,
                &hashed_merkle_tree,
                &cpi_ix_data.output_leaf_indices[j].into(),
                is_batched,
            )
            .map_err(ProgramError::from)?;
        cpi_ix_data.leaves[j].account_index = index_merkle_tree_account_account - 1;

        if !cpi_ix_data.nullifiers.is_empty() {
            if j == 0 {
                hash_chain = cpi_ix_data.leaves[j].leaf;
            } else {
                hash_chain = Poseidon::hashv(&[&hash_chain, &cpi_ix_data.leaves[j].leaf])
                    .map_err(ProgramError::from)?;
            }
        }
        context.set_rollover_fee(current_index as u8, rollover_fee);
    }

    cpi_ix_data.num_output_queues = index_merkle_tree_account as u8;
    Ok(hash_chain)
}
