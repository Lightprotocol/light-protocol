use crate::create_and_send_transaction_with_event;
use crate::test_env::NOOP_PROGRAM_ID;
use crate::test_indexer::AddressMerkleTreeBundle;
use crate::{get_hash_set, test_indexer::StateMerkleTreeBundle, AccountZeroCopy};
use account_compression::instruction::UpdateAddressMerkleTree;
use account_compression::utils::constants::ADDRESS_MERKLE_TREE_ROOTS;
use account_compression::{instruction::InsertAddresses, StateMerkleTreeAccount, ID};
use account_compression::{AddressMerkleTreeAccount, AddressQueueAccount};
use anchor_lang::system_program;
use anchor_lang::{InstructionData, ToAccountMetas};
use light_concurrent_merkle_tree::event::ChangelogEvent;
use light_hasher::Poseidon;
use light_utils::bigint::bigint_to_be_bytes_array;
use solana_program_test::{BanksClientError, ProgramTestContext};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use thiserror::Error;

// doesn't keep it's own Merkle tree but gets it from the indexer
// can also get all the state and Address Merkle trees from the indexer
// the lightweight version is just a function
// we should have a random option that shuffles the order in which to nullify transactions
// we should have a parameters how many to nullify
// in the test we should nullify everything once the queue is 60% full

/// Check compressed_accounts in the queue array which are not nullified yet
/// Iterate over these compressed_accounts and nullify them
/// Checks:
/// 1. Value in hashset is marked
/// 2. State tree root is updated
/// 3. TODO: add event is emitted (after rebase)
/// optional: assert that the Merkle tree doesn't change except the updated leaf
pub async fn nullify_compressed_accounts(
    context: &mut ProgramTestContext,
    payer: &Keypair,
    state_tree_bundle: &mut StateMerkleTreeBundle,
) {
    let nullifier_queue = unsafe {
        get_hash_set::<u16, account_compression::initialize_nullifier_queue::NullifierQueueAccount>(
            context,
            state_tree_bundle.accounts.nullifier_queue,
        )
        .await
    };
    let merkle_tree_account = AccountZeroCopy::<StateMerkleTreeAccount>::new(
        context,
        state_tree_bundle.accounts.merkle_tree,
    )
    .await;
    let onchain_merkle_tree = merkle_tree_account
        .deserialized()
        .copy_merkle_tree()
        .unwrap();
    assert_eq!(
        onchain_merkle_tree.root(),
        state_tree_bundle.merkle_tree.root()
    );
    let change_log_index = onchain_merkle_tree.changelog_index() as u64;

    let mut compressed_account_to_nullify = Vec::new();
    println!("\n --------------------------------------------------\n\t\t NULLIFYING LEAVES\n --------------------------------------------------");
    for (i, element) in nullifier_queue.iter() {
        if element.sequence_number().is_none() {
            println!("element to nullify: {:?}", element.value_bytes());
            let leaf_index: usize = state_tree_bundle
                .merkle_tree
                .get_leaf_index(&element.value_bytes())
                .unwrap();
            println!("leaf_index: {:?}", leaf_index);
            compressed_account_to_nullify.push((i, element.value_bytes()));
        }
    }

    for (index_in_nullifier_queue, compressed_account) in compressed_account_to_nullify.iter() {
        let leaf_index: usize = state_tree_bundle
            .merkle_tree
            .get_leaf_index(compressed_account)
            .unwrap();
        let proof: Vec<[u8; 32]> = state_tree_bundle
            .merkle_tree
            .get_proof_of_leaf(leaf_index, false)
            .unwrap()
            .to_array::<16>()
            .unwrap()
            .to_vec();

        let instructions = [
            account_compression::nullify_leaves::sdk_nullify::create_nullify_instruction(
                vec![change_log_index].as_slice(),
                vec![(*index_in_nullifier_queue) as u16].as_slice(),
                vec![leaf_index as u64].as_slice(),
                vec![proof].as_slice(),
                &payer.pubkey(),
                &state_tree_bundle.accounts.merkle_tree,
                &state_tree_bundle.accounts.nullifier_queue,
            ),
        ];

        let event = create_and_send_transaction_with_event::<ChangelogEvent>(
            context,
            &instructions,
            &payer.pubkey(),
            &[payer],
            None,
        )
        .await
        .unwrap()
        .unwrap();

        match event {
            ChangelogEvent::V2(event) => {
                assert_eq!(event.id, state_tree_bundle.accounts.merkle_tree.to_bytes());
                assert_eq!(event.seq, onchain_merkle_tree.sequence_number as u64 + 1);
                assert_eq!(event.leaves.len(), 1);
                assert_eq!(event.leaves[0].leaf, [0u8; 32]);
                assert_eq!(event.leaves[0].leaf_index, leaf_index as u64);
            }
            _ => {
                panic!("Wrong event type.");
            }
        }

        assert_value_is_marked_in_queue(
            context,
            state_tree_bundle,
            index_in_nullifier_queue,
            &onchain_merkle_tree,
            compressed_account,
        )
        .await;
    }
    // Locally nullify all leaves
    for (_, compressed_account) in compressed_account_to_nullify.iter() {
        let leaf_index = state_tree_bundle
            .merkle_tree
            .get_leaf_index(compressed_account)
            .unwrap();
        state_tree_bundle
            .merkle_tree
            .update(&[0u8; 32], leaf_index)
            .unwrap();
    }
    let merkle_tree_account = AccountZeroCopy::<StateMerkleTreeAccount>::new(
        context,
        state_tree_bundle.accounts.merkle_tree,
    )
    .await;
    let onchain_merkle_tree = merkle_tree_account
        .deserialized()
        .copy_merkle_tree()
        .unwrap();
    assert_eq!(
        onchain_merkle_tree.root(),
        state_tree_bundle.merkle_tree.root()
    );
}

async fn assert_value_is_marked_in_queue<'a>(
    context: &mut ProgramTestContext,
    state_tree_bundle: &mut StateMerkleTreeBundle,
    index_in_nullifier_queue: &usize,
    onchain_merkle_tree: &light_concurrent_merkle_tree::ConcurrentMerkleTree<
        'a,
        light_hasher::Poseidon,
        26,
    >,
    compressed_account: &[u8; 32],
) {
    let nullifier_queue = unsafe {
        get_hash_set::<u16, account_compression::initialize_nullifier_queue::NullifierQueueAccount>(
            context,
            state_tree_bundle.accounts.nullifier_queue,
        )
        .await
    };
    let array_element = nullifier_queue
        .by_value_index(
            *index_in_nullifier_queue,
            Some(onchain_merkle_tree.sequence_number),
        )
        .unwrap();
    assert_eq!(&array_element.value_bytes(), compressed_account);
    let merkle_tree_account = AccountZeroCopy::<StateMerkleTreeAccount>::new(
        context,
        state_tree_bundle.accounts.merkle_tree,
    )
    .await;
    assert_eq!(
        array_element.sequence_number(),
        Some(
            merkle_tree_account
                .deserialized()
                .load_merkle_tree()
                .unwrap()
                .sequence_number
                + account_compression::utils::constants::STATE_MERKLE_TREE_ROOTS as usize
        )
    );
}

#[derive(Error, Debug)]
pub enum RelayerUpdateError {}
/// Mocks the address insert logic of a forester.
/// Gets addresses from the AddressQueue and inserts them into the AddressMerkleTree.
/// Checks:
/// 1. Element has been marked correctly
/// 2. Merkle tree has been updated correctly
/// TODO: Event has been emitted, event doesn't exist yet
pub async fn empty_address_queue_test<const INDEXED_ARRAY_SIZE: usize>(
    context: &mut ProgramTestContext,
    address_tree_bundle: &mut AddressMerkleTreeBundle<INDEXED_ARRAY_SIZE>,
) -> Result<(), RelayerUpdateError> {
    let address_merkle_tree_pubkey = address_tree_bundle.accounts.merkle_tree;
    let address_queue_pubkey = address_tree_bundle.accounts.queue;
    let relayer_merkle_tree = &mut address_tree_bundle.merkle_tree;
    let relayer_indexing_array = &mut address_tree_bundle.indexed_array;
    let mut update_errors: Vec<BanksClientError> = Vec::new();

    loop {
        let address_merkle_tree =
            AccountZeroCopy::<AddressMerkleTreeAccount>::new(context, address_merkle_tree_pubkey)
                .await;
        let address_merkle_tree_deserialized = *address_merkle_tree.deserialized();
        let address_merkle_tree = address_merkle_tree_deserialized.copy_merkle_tree().unwrap();
        assert_eq!(
            relayer_merkle_tree.root(),
            address_merkle_tree.indexed_merkle_tree().root(),
        );
        let address_queue = unsafe {
            get_hash_set::<u16, AddressQueueAccount>(context, address_queue_pubkey).await
        };

        let address = address_queue.first_no_seq().unwrap();
        if address.is_none() {
            break;
        }
        let (address, address_hashset_index) = address.unwrap();
        // Create new element from the dequeued value.
        let (old_low_address, old_low_address_next_value) = relayer_indexing_array
            .find_low_element(&address.value_biguint())
            .unwrap();
        let address_bundle = relayer_indexing_array
            .new_element_with_low_element_index(old_low_address.index, &address.value_biguint())
            .unwrap();

        // Get the Merkle proof for updating low element.
        let low_address_proof = relayer_merkle_tree
            .get_proof_of_leaf(old_low_address.index, false)
            .unwrap();

        let old_sequence_number = address_merkle_tree
            .indexed_merkle_tree()
            .merkle_tree
            .sequence_number;
        let old_root = address_merkle_tree.indexed_merkle_tree().merkle_tree.root();
        // Update on-chain tree.
        let update_successful = match update_merkle_tree(
            context,
            address_queue_pubkey,
            address_merkle_tree_pubkey,
            address_hashset_index,
            address_bundle.new_element.next_index as u64,
            old_low_address.index as u64,
            bigint_to_be_bytes_array(&old_low_address.value).unwrap(),
            old_low_address.next_index as u64,
            bigint_to_be_bytes_array(&old_low_address_next_value).unwrap(),
            low_address_proof.to_array().unwrap(),
        )
        .await
        {
            Ok(event) => {
                let event = event.unwrap();
                match event {
                    ChangelogEvent::V2(event) => {
                        assert_eq!(event.id, address_merkle_tree_pubkey.to_bytes());
                        assert_eq!(event.seq, old_sequence_number as u64 + 1);
                        assert_eq!(event.leaves.len(), 2);
                        let new_low_element_leaf = address_bundle
                            .new_low_element
                            .hash::<Poseidon>(&address_bundle.new_element.value);
                        assert_eq!(
                            event.leaves[0].leaf,
                            new_low_element_leaf.unwrap(),
                            "New low element leaf mismatch."
                        );
                        let new_element_leaf = address_bundle
                            .new_element
                            .hash::<Poseidon>(&address_bundle.new_element_next_value);
                        assert_eq!(
                            event.leaves[1].leaf,
                            new_element_leaf.unwrap(),
                            "New element leaf mismatch."
                        );
                        assert_eq!(event.leaves[0].leaf_index, old_low_address.index as u64);
                        assert_eq!(
                            event.leaves[1].leaf_index,
                            address_bundle.new_element.index as u64
                        );
                    }
                    _ => {
                        panic!("Wrong event type.");
                    }
                }

                true
            }
            Err(e) => {
                update_errors.push(e);
                break;
            }
        };

        if update_successful {
            let merkle_tree_account = AccountZeroCopy::<AddressMerkleTreeAccount>::new(
                context,
                address_merkle_tree_pubkey,
            )
            .await;
            let merkle_tree = merkle_tree_account
                .deserialized()
                .copy_merkle_tree()
                .unwrap();
            let address_queue = unsafe {
                get_hash_set::<u16, AddressQueueAccount>(context, address_queue_pubkey).await
            };

            assert_eq!(
                address_queue
                    .by_value_index(address_hashset_index as usize, Some(0))
                    .unwrap()
                    .sequence_number()
                    .unwrap(),
                old_sequence_number + ADDRESS_MERKLE_TREE_ROOTS as usize
            );

            relayer_merkle_tree
                .update(
                    &address_bundle.new_low_element,
                    &address_bundle.new_element,
                    &address_bundle.new_element_next_value,
                )
                .unwrap();
            relayer_indexing_array
                .append_with_low_element_index(
                    address_bundle.new_low_element.index,
                    &address_bundle.new_element.value,
                )
                .unwrap();
            assert_eq!(
                merkle_tree
                    .indexed_merkle_tree()
                    .merkle_tree
                    .sequence_number,
                old_sequence_number + 2
            );
            assert_ne!(
                old_root,
                merkle_tree.indexed_merkle_tree().merkle_tree.root(),
                "Root did not change."
            );
            assert_eq!(
                relayer_merkle_tree.root(),
                merkle_tree.indexed_merkle_tree().merkle_tree.root(),
                "Root offchain onchain inconsistent."
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
pub async fn update_merkle_tree(
    context: &mut ProgramTestContext,
    address_queue_pubkey: Pubkey,
    address_merkle_tree_pubkey: Pubkey,
    value: u16,
    next_index: u64,
    low_address_index: u64,
    low_address_value: [u8; 32],
    low_address_next_index: u64,
    low_address_next_value: [u8; 32],
    low_address_proof: [[u8; 32]; 16],
) -> Result<Option<ChangelogEvent>, BanksClientError> {
    let changelog_index = {
        // TODO: figure out why I get an invalid memory reference error here when I try to replace 183-190 with this
        let address_merkle_tree =
            AccountZeroCopy::<AddressMerkleTreeAccount>::new(context, address_merkle_tree_pubkey)
                .await;

        let address_merkle_tree = &address_merkle_tree
            .deserialized()
            .load_merkle_tree()
            .unwrap();
        address_merkle_tree.merkle_tree.changelog_index()
    };

    let instruction_data = UpdateAddressMerkleTree {
        changelog_index: changelog_index as u16,
        value,
        next_index,
        low_address_index,
        low_address_value,
        low_address_next_index,
        low_address_next_value,
        low_address_proof,
    };
    let update_ix = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(context.payer.pubkey(), true),
            AccountMeta::new(address_queue_pubkey, false),
            AccountMeta::new(address_merkle_tree_pubkey, false),
            AccountMeta::new(NOOP_PROGRAM_ID, false),
        ],
        data: instruction_data.data(),
    };
    let payer = context.payer.insecure_clone();
    create_and_send_transaction_with_event::<ChangelogEvent>(
        context,
        &[update_ix],
        &context.payer.pubkey(),
        &[&payer],
        None,
    )
    .await
}

pub async fn insert_addresses(
    context: &mut ProgramTestContext,
    address_queue_pubkey: Pubkey,
    address_merkle_tree_pubkey: Pubkey,
    addresses: Vec<[u8; 32]>,
) -> Result<(), BanksClientError> {
    let num_addresses = addresses.len();
    let instruction_data = InsertAddresses { addresses };
    let accounts = account_compression::accounts::InsertAddresses {
        fee_payer: context.payer.pubkey(),
        authority: context.payer.pubkey(),
        registered_program_pda: None,
        system_program: system_program::ID,
    };
    let insert_ix = Instruction {
        program_id: ID,
        accounts: [
            accounts.to_account_metas(Some(true)),
            vec![
                vec![
                    AccountMeta::new(address_queue_pubkey, false),
                    AccountMeta::new(address_merkle_tree_pubkey, false),
                ];
                num_addresses
            ]
            .iter()
            .flat_map(|x| x.to_vec())
            .collect::<Vec<AccountMeta>>(),
        ]
        .concat(),
        data: instruction_data.data(),
    };
    let transaction = Transaction::new_signed_with_payer(
        &[insert_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &context.payer],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(transaction).await
}
