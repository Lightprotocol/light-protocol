use light_compressed_account::{
    hash_to_bn254_field_size_be,
    instruction_data::{
        insert_into_queues::{InsertIntoQueuesInstructionDataMut, MerkleTreeSequenceNumber},
        traits::InstructionData,
    },
    TreeType,
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
#[profile]
pub fn create_outputs_cpi_data<'a, 'info, T: InstructionData<'a>>(
    inputs: &WrappedInstructionData<'a, T>,
    remaining_accounts: &'info [AccountInfo],
    context: &mut SystemContext<'info>,
    cpi_ix_data: &mut InsertIntoQueuesInstructionDataMut<'_>,
    accounts: &[AcpAccount<'info>],
) -> Result<[u8; 32]> {
    if inputs.output_len() == 0 {
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
        inputs.output_accounts().last().unwrap().merkle_tree_index() as usize + 1;

    let mut merkle_tree_pubkeys =
        Vec::<light_compressed_account::pubkey::Pubkey>::with_capacity(number_of_merkle_trees);
    let mut hash_chain = [0u8; 32];
    let mut rollover_fee = 0;
    let mut is_batched = true;

    for (j, account) in inputs.output_accounts().enumerate() {
        // if mt index == current index Merkle tree account info has already been added.
        // if mt index != current index, Merkle tree account info is new, add it.
        #[allow(clippy::comparison_chain)]
        if account.merkle_tree_index() as i16 == current_index {
            // Do nothing, but it is the most common case.
        } else if account.merkle_tree_index() as i16 > current_index {
            current_index = account.merkle_tree_index().into();

            let pubkey = match &accounts
                .get(current_index as usize)
                .ok_or(SystemProgramError::OutputMerkleTreeIndexOutOfBounds)?
            {
                AcpAccount::OutputQueue(output_queue) => {
                    context.set_network_fee_v2(
                        output_queue.metadata.rollover_metadata.network_fee,
                        current_index as u8,
                    );

                    hashed_merkle_tree = output_queue.hashed_merkle_tree_pubkey;
                    rollover_fee = output_queue.metadata.rollover_metadata.rollover_fee;
                    mt_next_index = output_queue.batch_metadata.next_index as u32;
                    cpi_ix_data.output_sequence_numbers[index_merkle_tree_account as usize] =
                        MerkleTreeSequenceNumber {
                            tree_pubkey: output_queue.metadata.associated_merkle_tree,
                            queue_pubkey: *output_queue.pubkey(),
                            tree_type: (TreeType::StateV2 as u64).into(),
                            seq: output_queue.batch_metadata.next_index.into(),
                        };
                    is_batched = true;
                    *output_queue.pubkey()
                }
                AcpAccount::StateTree((pubkey, tree)) => {
                    cpi_ix_data.output_sequence_numbers[index_merkle_tree_account as usize] =
                        MerkleTreeSequenceNumber {
                            tree_pubkey: *pubkey,
                            queue_pubkey: *pubkey,
                            tree_type: (TreeType::StateV1 as u64).into(),
                            seq: (tree.sequence_number() as u64 + 1).into(),
                        };
                    let merkle_context = context
                        .get_legacy_merkle_context(current_index as u8)
                        .ok_or(SystemProgramError::MissingLegacyMerkleContext)?;
                    hashed_merkle_tree = merkle_context.hashed_pubkey;
                    rollover_fee = merkle_context.rollover_fee;
                    mt_next_index = tree.next_index() as u32;
                    is_batched = false;
                    *pubkey
                }
                AcpAccount::Unknown() => {
                    msg!(
                        format!("found batched unknown create outputs {} ", current_index).as_str()
                    );

                    return Err(
                        SystemProgramError::StateMerkleTreeAccountDiscriminatorMismatch.into(),
                    );
                }
                AcpAccount::BatchedAddressTree(_) => {
                    msg!(format!(
                        "found batched address tree create outputs {} ",
                        current_index
                    )
                    .as_str());

                    return Err(
                        SystemProgramError::StateMerkleTreeAccountDiscriminatorMismatch.into(),
                    );
                }
                AcpAccount::BatchedStateTree(_) => {
                    msg!(
                        format!("found batched state tree create outputs {} ", current_index)
                            .as_str()
                    );

                    return Err(
                        SystemProgramError::StateMerkleTreeAccountDiscriminatorMismatch.into(),
                    );
                }
                _ => {
                    msg!(format!("create outputs {} ", current_index).as_str());

                    return Err(
                        SystemProgramError::StateMerkleTreeAccountDiscriminatorMismatch.into(),
                    );
                }
            };
            // check Merkle tree uniqueness
            if merkle_tree_pubkeys.contains(&pubkey) {
                return Err(SystemProgramError::OutputMerkleTreeNotUnique.into());
            } else {
                merkle_tree_pubkeys.push(pubkey);
            }

            context.get_index_or_insert(
                account.merkle_tree_index(),
                remaining_accounts,
                "Output queue for V2 state trees (Merkle tree for V1 state trees)",
            )?;
            num_leaves_in_tree = 0;
            index_merkle_tree_account += 1;
            index_merkle_tree_account_account += 1;
        } else {
            // Check 2.
            // Output Merkle tree indices must be in order since we use the
            // number of leaves in a Merkle tree to determine the correct leaf
            // index. Since the leaf index is part of the hash this is security
            // critical.
            return Err(SystemProgramError::OutputMerkleTreeIndicesNotInOrder.into());
        }

        // Check 3.
        if let Some(address) = account.address() {
            if let Some(position) = context
                .addresses
                .iter()
                .filter(|x| x.is_some())
                .position(|&x| x.unwrap() == address)
            {
                context.addresses.remove(position);
            } else {
                msg!(format!("context.addresses: {:?}", context.addresses).as_str());
                return Err(SystemProgramError::InvalidAddress.into());
            }
        }
        cpi_ix_data.output_leaf_indices[j] = (mt_next_index + num_leaves_in_tree).into();

        num_leaves_in_tree += 1;
        if account.has_data() && context.invoking_program_id.is_none() {
            msg!("Invoking program is not provided.");
            msg!("Only program owned compressed accounts can have data.");
            return Err(SystemProgramError::InvokingProgramNotProvided.into());
        }
        let hashed_owner = match context
            .hashed_pubkeys
            .iter()
            .find(|x| x.0 == account.owner().to_bytes())
        {
            Some(hashed_owner) => hashed_owner.1,
            None => {
                let hashed_owner = hash_to_bn254_field_size_be(&account.owner().to_bytes());
                context
                    .hashed_pubkeys
                    .push((account.owner().into(), hashed_owner));
                hashed_owner
            }
        };
        // Compute output compressed account hash.
        cpi_ix_data.leaves[j].leaf = account
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

// Check that new addresses are assigned correctly to the compressed output accounts specified by index
#[profile]
pub fn check_new_address_assignment<'a, 'info, T: InstructionData<'a>>(
    inputs: &WrappedInstructionData<'a, T>,
    cpi_ix_data: &InsertIntoQueuesInstructionDataMut<'_>,
) -> std::result::Result<(), SystemProgramError> {
    for (derived_addresses, new_addresses) in
        cpi_ix_data.addresses.iter().zip(inputs.new_addresses())
    {
        if let Some(assigned_account_index) = new_addresses.assigned_compressed_account_index() {
            let output_account = inputs
                .get_output_account(assigned_account_index)
                .ok_or(SystemProgramError::NewAddressAssignedIndexOutOfBounds)?;

            if derived_addresses.address
                != output_account
                    .address()
                    .ok_or(SystemProgramError::AddressIsNone)?
            {
                msg!(format!(
                    "derived_addresses.address {:?} != account address {:?}",
                    derived_addresses.address,
                    output_account.address()
                )
                .as_str());
                msg!(format!(
                    "account owner {:?}",
                    solana_pubkey::Pubkey::new_from_array(output_account.owner().into())
                )
                .as_str());
                msg!(format!(
                    "account merkle_tree_index {:?}",
                    output_account.merkle_tree_index()
                )
                .as_str());
                return Err(SystemProgramError::AddressDoesNotMatch);
            }
        }
    }
    Ok(())
}
