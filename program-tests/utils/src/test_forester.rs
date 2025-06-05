use account_compression::{
    instruction::UpdateAddressMerkleTree,
    state::QueueAccount,
    utils::constants::{ADDRESS_MERKLE_TREE_HEIGHT, ADDRESS_MERKLE_TREE_ROOTS},
    AddressMerkleTreeAccount, StateMerkleTreeAccount, ID, SAFETY_MARGIN,
};
use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use forester_utils::account_zero_copy::{
    get_concurrent_merkle_tree, get_hash_set, get_indexed_merkle_tree,
};
use light_client::rpc::{errors::RpcError, Rpc};
use light_concurrent_merkle_tree::event::MerkleTreeEvent;
use light_hasher::{bigint::bigint_to_be_bytes_array, Poseidon};
use light_indexed_merkle_tree::copy::IndexedMerkleTreeCopy;
use light_program_test::{
    accounts::test_accounts::NOOP_PROGRAM_ID,
    indexer::{address_tree::AddressMerkleTreeBundle, state_tree::StateMerkleTreeBundle},
    program_test::test_rpc::TestRpc,
    Indexer,
};
use light_registry::{
    account_compression_cpi::sdk::{
        create_nullify_instruction, create_update_address_merkle_tree_instruction,
        CreateNullifyInstructionInputs, UpdateAddressMerkleTreeInstructionInputs,
    },
    utils::get_forester_epoch_pda_from_authority,
    ForesterEpochPda, RegisterForester,
};
use log::debug;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};
use thiserror::Error;
// doesn't keep its own Merkle tree but gets it from the indexer
// can also get all the state and Address Merkle trees from the indexer
// the lightweight version is just a function
// we should have a random option that shuffles the order in which to nullify transactions
// we should have a parameters how many to nullify
// in the test we should nullify everything once the queue is 60% full

/// Check compressed_accounts in the queue array which are not nullified yet
/// Iterate over these compressed_accounts and nullify them
///
/// Checks:
/// 1. Value in hashset is marked
/// 2. State tree root is updated
/// 3. TODO: add event is emitted (after rebase)
///    optional: assert that the Merkle tree doesn't change except the updated leaf
pub async fn nullify_compressed_accounts<R: Rpc + TestRpc + Indexer + Indexer>(
    rpc: &mut R,
    forester: &Keypair,
    state_tree_bundle: &mut StateMerkleTreeBundle,
    epoch: u64,
    is_metadata_forester: bool,
) -> Result<(), RpcError> {
    let nullifier_queue = unsafe {
        get_hash_set::<QueueAccount, R>(rpc, state_tree_bundle.accounts.nullifier_queue).await
    };
    let pre_forester_counter = if is_metadata_forester {
        0
    } else {
        rpc.get_anchor_account::<ForesterEpochPda>(
            &get_forester_epoch_pda_from_authority(&forester.pubkey(), epoch).0,
        )
        .await
        .unwrap()
        .unwrap()
        .work_counter
    };
    let onchain_merkle_tree =
        get_concurrent_merkle_tree::<StateMerkleTreeAccount, R, Poseidon, 26>(
            rpc,
            state_tree_bundle.accounts.merkle_tree,
        )
        .await;
    assert_eq!(
        onchain_merkle_tree.root(),
        state_tree_bundle.merkle_tree.root()
    );
    let pre_root = onchain_merkle_tree.root();
    let change_log_index = onchain_merkle_tree.changelog_index() as u64;

    let mut compressed_account_to_nullify = Vec::new();

    let first = nullifier_queue.first_no_seq().unwrap();

    for i in 0..nullifier_queue.get_capacity() {
        let bucket = nullifier_queue.get_bucket(i).unwrap();
        if let Some(bucket) = bucket {
            if bucket.sequence_number.is_none() {
                debug!("element to nullify: {:?}", bucket.value_bytes());
                let leaf_index: usize = state_tree_bundle
                    .merkle_tree
                    .get_leaf_index(&bucket.value_bytes())
                    .unwrap();
                debug!("leaf_index: {:?}", leaf_index);
                compressed_account_to_nullify.push((i, bucket.value_bytes()));
            }
        }
    }

    debug!(
        "nullifying {:?} accounts ",
        compressed_account_to_nullify.len()
    );

    for (i, (index_in_nullifier_queue, compressed_account)) in
        compressed_account_to_nullify.iter().enumerate()
    {
        let leaf_index: usize = state_tree_bundle
            .merkle_tree
            .get_leaf_index(compressed_account)
            .unwrap();
        debug!("nullifying leaf: {:?}", leaf_index);

        let proof: Vec<[u8; 32]> = state_tree_bundle
            .merkle_tree
            .get_proof_of_leaf(leaf_index, false)
            .unwrap();
        let ix = create_nullify_instruction(
            CreateNullifyInstructionInputs {
                authority: forester.pubkey(),
                nullifier_queue: state_tree_bundle.accounts.nullifier_queue,
                merkle_tree: state_tree_bundle.accounts.merkle_tree,
                change_log_indices: vec![change_log_index],
                leaves_queue_indices: vec![*index_in_nullifier_queue as u16],
                indices: vec![leaf_index as u64],
                proofs: vec![proof],
                derivation: forester.pubkey(),
                is_metadata_forester,
            },
            epoch,
        );
        let instructions = [ix];

        let event = Rpc::create_and_send_transaction_with_event::<MerkleTreeEvent>(
            rpc,
            &instructions,
            &forester.pubkey(),
            &[forester],
        )
        .await?
        .unwrap();

        match event.0 {
            MerkleTreeEvent::V2(event) => {
                assert_eq!(event.id, state_tree_bundle.accounts.merkle_tree.to_bytes());
                assert_eq!(
                    event.seq,
                    onchain_merkle_tree.sequence_number() as u64 + 1 + i as u64
                );
                assert_eq!(event.nullified_leaves_indices.len(), 1);
                assert_eq!(event.nullified_leaves_indices[0], leaf_index as u64);
            }
            _ => {
                panic!("Wrong event type.");
            }
        }

        assert_value_is_marked_in_queue(
            rpc,
            state_tree_bundle,
            index_in_nullifier_queue,
            compressed_account,
        )
        .await;
    }

    let num_nullified = compressed_account_to_nullify.len() as u64;
    // Locally nullify all leaves
    for (_, compressed_account) in compressed_account_to_nullify.iter() {
        let leaf_index = state_tree_bundle
            .merkle_tree
            .get_leaf_index(compressed_account)
            .unwrap();
        debug!("locally nullifying leaf_index {}", leaf_index);
        debug!("compressed_account {:?}", compressed_account);
        debug!(
            "merkle tree pubkey {:?}",
            state_tree_bundle.accounts.merkle_tree
        );

        state_tree_bundle
            .merkle_tree
            .update(&[0u8; 32], leaf_index)
            .unwrap();
    }
    let onchain_merkle_tree =
        get_concurrent_merkle_tree::<StateMerkleTreeAccount, R, Poseidon, 26>(
            rpc,
            state_tree_bundle.accounts.merkle_tree,
        )
        .await;
    assert_eq!(
        onchain_merkle_tree.root(),
        state_tree_bundle.merkle_tree.root()
    );
    if !is_metadata_forester {
        assert_forester_counter(
            rpc,
            &get_forester_epoch_pda_from_authority(&forester.pubkey(), epoch).0,
            pre_forester_counter,
            num_nullified,
        )
        .await
        .unwrap();
    }
    // SAFEGUARD: check that the root changed if there was at least one element to nullify
    if first.is_some() {
        assert_ne!(pre_root, onchain_merkle_tree.root());
    }
    Ok(())
}

async fn assert_value_is_marked_in_queue<R: Rpc>(
    rpc: &mut R,
    state_tree_bundle: &mut StateMerkleTreeBundle,
    index_in_nullifier_queue: &usize,
    compressed_account: &[u8; 32],
) {
    let nullifier_queue = unsafe {
        get_hash_set::<QueueAccount, R>(rpc, state_tree_bundle.accounts.nullifier_queue).await
    };
    let array_element = nullifier_queue
        .get_bucket(*index_in_nullifier_queue)
        .unwrap()
        .unwrap();
    assert_eq!(&array_element.value_bytes(), compressed_account);
    let onchain_merkle_tree =
        get_concurrent_merkle_tree::<StateMerkleTreeAccount, R, Poseidon, 26>(
            rpc,
            state_tree_bundle.accounts.merkle_tree,
        )
        .await;
    assert_eq!(
        array_element.sequence_number(),
        Some(
            onchain_merkle_tree.sequence_number()
                + onchain_merkle_tree.roots.capacity()
                + SAFETY_MARGIN as usize
        )
    );
}

pub async fn assert_forester_counter<R: Rpc>(
    rpc: &mut R,
    pubkey: &Pubkey,
    pre: u64,
    num_nullified: u64,
) -> Result<(), RpcError> {
    let account = rpc
        .get_anchor_account::<ForesterEpochPda>(pubkey)
        .await?
        .unwrap();
    if account.work_counter != pre + num_nullified {
        debug!("account.work_counter: {}", account.work_counter);
        debug!("pre: {}", pre);
        debug!("num_nullified: {}", num_nullified);
        debug!("forester pubkey: {:?}", pubkey);
        return Err(RpcError::CustomError(
            "ForesterEpochPda counter not updated correctly".to_string(),
        ));
    }
    Ok(())
}

#[derive(Error, Debug)]
pub enum RelayerUpdateError {
    #[error("Error in relayer update")]
    RpcError,
}
/// Mocks the address insert logic of a forester.
/// Gets addresses from the AddressQueue and inserts them into the AddressMerkleTree.
///
/// Checks:
/// 1. Element has been marked correctly
/// 2. Merkle tree has been updated correctly
///
/// TODO: Event has been emitted, event doesn't exist yet
pub async fn empty_address_queue_test<R: Rpc>(
    forester: &Keypair,
    rpc: &mut R,
    address_tree_bundle: &mut AddressMerkleTreeBundle,
    signer_is_owner: bool,
    epoch: u64,
    is_metadata_forester: bool,
) -> Result<(), RelayerUpdateError> {
    let address_merkle_tree_pubkey = address_tree_bundle.accounts.merkle_tree;
    let address_queue_pubkey = address_tree_bundle.accounts.queue;
    let initial_merkle_tree_state = address_tree_bundle
        .get_v1_indexed_merkle_tree()
        .unwrap()
        .clone();
    let initial_indexed_array_state = address_tree_bundle.indexed_array_v1().unwrap().clone();
    let mut update_errors: Vec<RpcError> = Vec::new();
    let address_merkle_tree =
        get_indexed_merkle_tree::<AddressMerkleTreeAccount, R, Poseidon, usize, 26, 16>(
            rpc,
            address_merkle_tree_pubkey,
        )
        .await;
    let indexed_changelog_index = address_merkle_tree.indexed_changelog_index() as u16;
    let changelog_index = address_merkle_tree.changelog_index() as u16;
    let mut counter = 0;
    loop {
        let pre_forester_counter = if !signer_is_owner {
            rpc.get_anchor_account::<ForesterEpochPda>(
                &get_forester_epoch_pda_from_authority(&forester.pubkey(), epoch).0,
            )
            .await
            .map_err(|e| RelayerUpdateError::RpcError)?
            .unwrap()
            .work_counter
        } else {
            0
        };
        let address_merkle_tree =
            get_indexed_merkle_tree::<AddressMerkleTreeAccount, R, Poseidon, usize, 26, 16>(
                rpc,
                address_merkle_tree_pubkey,
            )
            .await;
        assert_eq!(address_tree_bundle.root(), address_merkle_tree.root());
        let address_queue =
            unsafe { get_hash_set::<QueueAccount, R>(rpc, address_queue_pubkey).await };

        let address = address_queue.first_no_seq().unwrap();

        if address.is_none() {
            break;
        }
        let (address, address_hashset_index) = address.unwrap();
        // Create new element from the dequeued value.
        let (old_low_address, old_low_address_next_value) = initial_indexed_array_state
            .find_low_element_for_nonexistent(&address.value_biguint())
            .unwrap();
        let address_bundle = initial_indexed_array_state
            .new_element_with_low_element_index(old_low_address.index, &address.value_biguint())
            .unwrap();

        // Get the Merkle proof for updating low element.
        let low_address_proof = initial_merkle_tree_state
            .get_proof_of_leaf(old_low_address.index, false)
            .unwrap();

        let old_sequence_number = address_merkle_tree.sequence_number();
        let old_root = address_merkle_tree.root();
        // Update on-chain tree.
        let update_successful = match update_merkle_tree(
            rpc,
            forester,
            address_queue_pubkey,
            address_merkle_tree_pubkey,
            address_hashset_index,
            old_low_address.index as u64,
            bigint_to_be_bytes_array(&old_low_address.value).unwrap(),
            old_low_address.next_index as u64,
            bigint_to_be_bytes_array(&old_low_address_next_value).unwrap(),
            low_address_proof.to_array().unwrap(),
            Some(changelog_index),
            Some(indexed_changelog_index),
            signer_is_owner,
            epoch,
            is_metadata_forester,
        )
        .await
        {
            Ok(event) => {
                let event = event.unwrap();
                match event.0 {
                    MerkleTreeEvent::V3(event) => {
                        // Only assert for the first update since the other updates might be patched
                        // the asserts are likely to fail
                        if counter == 0 {
                            assert_eq!(event.id, address_merkle_tree_pubkey.to_bytes());
                            assert_eq!(event.seq, old_sequence_number as u64 + 1);
                            assert_eq!(event.updates.len(), 1);
                            let event = &event.updates[0];
                            assert_eq!(
                                event.new_low_element.index, address_bundle.new_low_element.index,
                                "Empty Address Queue Test: invalid new_low_element.index"
                            );
                            assert_eq!(
                                event.new_low_element.next_index,
                                address_bundle.new_low_element.next_index,
                                "Empty Address Queue Test: invalid new_low_element.next_index"
                            );
                            assert_eq!(
                                event.new_low_element.value,
                                bigint_to_be_bytes_array::<32>(
                                    &address_bundle.new_low_element.value
                                )
                                .unwrap(),
                                "Empty Address Queue Test: invalid new_low_element.value"
                            );
                            assert_eq!(
                                event.new_low_element.next_value,
                                bigint_to_be_bytes_array::<32>(&address_bundle.new_element.value)
                                    .unwrap(),
                                "Empty Address Queue Test: invalid new_low_element.next_value"
                            );
                            let leaf_hash = address_bundle
                                .new_low_element
                                .hash::<Poseidon>(&address_bundle.new_element.value)
                                .unwrap();
                            assert_eq!(
                                event.new_low_element_hash, leaf_hash,
                                "Empty Address Queue Test: invalid new_low_element_hash"
                            );
                            let leaf_hash = address_bundle
                                .new_element
                                .hash::<Poseidon>(&address_bundle.new_element_next_value)
                                .unwrap();
                            assert_eq!(
                                event.new_high_element_hash, leaf_hash,
                                "Empty Address Queue Test: invalid new_high_element_hash"
                            );
                            assert_eq!(
                                event.new_high_element.index, address_bundle.new_element.index,
                                "Empty Address Queue Test: invalid new_high_element.index"
                            );
                            assert_eq!(
                                event.new_high_element.next_index,
                                address_bundle.new_element.next_index,
                                "Empty Address Queue Test: invalid new_high_element.next_index"
                            );
                            assert_eq!(
                                event.new_high_element.value,
                                bigint_to_be_bytes_array::<32>(&address_bundle.new_element.value)
                                    .unwrap(),
                                "Empty Address Queue Test: invalid new_high_element.value"
                            );
                            assert_eq!(
                                event.new_high_element.next_value,
                                bigint_to_be_bytes_array::<32>(
                                    &address_bundle.new_element_next_value
                                )
                                .unwrap(),
                                "Empty Address Queue Test: invalid new_high_element.next_value"
                            );
                        }
                    }
                    _ => {
                        panic!("Wrong event type.");
                    }
                }
                counter += 1;
                true
            }
            Err(e) => {
                update_errors.push(e);
                break;
            }
        };

        if update_successful {
            if !signer_is_owner {
                assert_forester_counter(
                    rpc,
                    &get_forester_epoch_pda_from_authority(&forester.pubkey(), epoch).0,
                    pre_forester_counter,
                    1,
                )
                .await
                .unwrap();
            }
            let merkle_tree =
                get_indexed_merkle_tree::<AddressMerkleTreeAccount, R, Poseidon, usize, 26, 16>(
                    rpc,
                    address_merkle_tree_pubkey,
                )
                .await;

            let (old_low_address, _) = address_tree_bundle
                .find_low_element_for_nonexistent(&address.value_biguint())
                .unwrap();
            let address_bundle = address_tree_bundle
                .new_element_with_low_element_index(old_low_address.index, &address.value_biguint())
                .unwrap();
            let address_queue =
                unsafe { get_hash_set::<QueueAccount, R>(rpc, address_queue_pubkey).await };

            assert_eq!(
                address_queue
                    .get_bucket(address_hashset_index as usize)
                    .unwrap()
                    .unwrap()
                    .sequence_number()
                    .unwrap(),
                old_sequence_number + address_queue.sequence_threshold + 2 // We are doing two Merkle tree operations
            );

            address_tree_bundle
                .update(
                    &address_bundle.new_low_element,
                    &address_bundle.new_element,
                    &address_bundle.new_element_next_value,
                )
                .unwrap();
            address_tree_bundle
                .append_with_low_element_index(
                    address_bundle.new_low_element.index,
                    &address_bundle.new_element.value,
                )
                .unwrap();
            assert_eq!(merkle_tree.sequence_number(), old_sequence_number + 2);
            assert_ne!(old_root, merkle_tree.root(), "Root did not change.");
            assert_eq!(
                address_tree_bundle.root(),
                merkle_tree.root(),
                "Root off-chain onchain inconsistent."
            );

            let changelog_entry = merkle_tree
                .changelog
                .get(merkle_tree.changelog_index())
                .unwrap();
            let path = address_tree_bundle
                .get_path_of_leaf(merkle_tree.current_index(), true)
                .unwrap();
            for (i, path_node) in path.iter().enumerate() {
                let changelog_node = changelog_entry.path[i].unwrap();
                assert_eq!(changelog_node, *path_node);
            }

            let indexed_changelog_entry = merkle_tree
                .indexed_changelog
                .get(merkle_tree.indexed_changelog_index())
                .unwrap();
            let proof = address_tree_bundle
                .get_proof_of_leaf(merkle_tree.current_index(), false)
                .unwrap();
            assert_eq!(
                address_bundle.new_element,
                indexed_changelog_entry.element.into(),
            );
            assert_eq!(indexed_changelog_entry.proof.as_slice(), proof.as_slice());
            assert_eq!(
                indexed_changelog_entry.changelog_index,
                merkle_tree.changelog_index()
            );
        }
    }

    if update_errors.is_empty() {
        Ok(())
    } else {
        panic!("Errors: {:?}", update_errors);
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn update_merkle_tree<R: Rpc>(
    rpc: &mut R,
    forester: &Keypair,
    address_queue_pubkey: Pubkey,
    address_merkle_tree_pubkey: Pubkey,
    value: u16,
    low_address_index: u64,
    low_address_value: [u8; 32],
    low_address_next_index: u64,
    low_address_next_value: [u8; 32],
    low_address_proof: [[u8; 32]; 16],
    changelog_index: Option<u16>,
    indexed_changelog_index: Option<u16>,
    signer_is_owner: bool,
    epoch: u64,
    is_metadata_forester: bool,
) -> Result<Option<(MerkleTreeEvent, Signature, u64)>, RpcError> {
    let changelog_index = match changelog_index {
        Some(changelog_index) => changelog_index,
        None => {
            let address_merkle_tree =
                get_indexed_merkle_tree::<AddressMerkleTreeAccount, R, Poseidon, usize, 26, 16>(
                    rpc,
                    address_merkle_tree_pubkey,
                )
                .await;

            address_merkle_tree.changelog_index() as u16
        }
    };
    let indexed_changelog_index = match indexed_changelog_index {
        Some(indexed_changelog_index) => indexed_changelog_index,
        None => {
            let address_merkle_tree =
                get_indexed_merkle_tree::<AddressMerkleTreeAccount, R, Poseidon, usize, 26, 16>(
                    rpc,
                    address_merkle_tree_pubkey,
                )
                .await;

            address_merkle_tree.indexed_changelog_index() as u16
        }
    };
    let update_ix = if !signer_is_owner {
        create_update_address_merkle_tree_instruction(
            UpdateAddressMerkleTreeInstructionInputs {
                authority: forester.pubkey(),
                derivation: forester.pubkey(),
                address_merkle_tree: address_merkle_tree_pubkey,
                address_queue: address_queue_pubkey,
                changelog_index,
                indexed_changelog_index,
                value,
                low_address_index,
                low_address_value,
                low_address_next_index,
                low_address_next_value,
                low_address_proof,
                is_metadata_forester,
            },
            epoch,
        )
    } else {
        let instruction_data = UpdateAddressMerkleTree {
            changelog_index,
            indexed_changelog_index,
            value,
            low_address_index,
            low_address_value,
            low_address_next_index,
            low_address_next_value,
            low_address_proof,
        };
        Instruction {
            program_id: ID,
            accounts: vec![
                AccountMeta::new(forester.pubkey(), true),
                AccountMeta::new(ID, false),
                AccountMeta::new(address_queue_pubkey, false),
                AccountMeta::new(address_merkle_tree_pubkey, false),
                AccountMeta::new(NOOP_PROGRAM_ID, false),
            ],
            data: instruction_data.data(),
        }
    };

    rpc.create_and_send_transaction_with_event::<MerkleTreeEvent>(
        &[update_ix],
        &forester.pubkey(),
        &[forester],
    )
    .await
}
