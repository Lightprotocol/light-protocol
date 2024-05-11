use account_compression::StateMerkleTreeAccount;
use light_hasher::Poseidon;
use light_indexed_merkle_tree::{array::IndexedArray, IndexedMerkleTree};
use light_merkle_tree_reference::MerkleTree;
use solana_program_test::ProgramTestContext;
use solana_sdk::signature::{Keypair, Signer};

use crate::{
    create_and_send_transaction, get_hash_set,
    test_indexer::{AddressMerkleTreeAccounts, StateMerkleTreeAccounts},
    AccountZeroCopy,
};

///
///
///
#[derive(Debug)]
pub struct TestForester<'a> {
    pub state_merkle_trees: Vec<(StateMerkleTreeAccounts, MerkleTree<Poseidon>)>,
    pub address_merkle_trees: Vec<(
        AddressMerkleTreeAccounts,
        IndexedMerkleTree<'a, Poseidon, usize, 26>,
        IndexedArray<Poseidon, usize, 1000>,
    )>,
    pub payer: Keypair,
}
// doesn't keep it's own Merkle tree but gets it from the indexer
// can also get all the state and Address Merkle trees from the indexer
// the lightweight version is just a function
// we should have a random option that shuffles the order in which to nullify transactions
// we should have a parameters how many to nullify
// in the test we should nullify everything once the queue is 60% full

/// Check compressed_accounts in the queue array which are not nullified yet
/// Iterate over these compressed_accounts and nullify them
pub async fn nullify_compressed_accounts(
    context: &mut ProgramTestContext,
    payer: &Keypair,
    state_merkle_tree_accounts: &StateMerkleTreeAccounts,
    merkle_tree: &mut MerkleTree<Poseidon>,
) {
    let nullifier_queue = unsafe {
        get_hash_set::<u16, account_compression::initialize_nullifier_queue::NullifierQueueAccount>(
            context,
            state_merkle_tree_accounts.nullifier_queue,
        )
        .await
    };
    let merkle_tree_account = AccountZeroCopy::<StateMerkleTreeAccount>::new(
        context,
        state_merkle_tree_accounts.merkle_tree,
    )
    .await;
    let onchain_merkle_tree = merkle_tree_account
        .deserialized()
        .copy_merkle_tree()
        .unwrap();
    assert_eq!(onchain_merkle_tree.root().unwrap(), merkle_tree.root());
    let change_log_index = onchain_merkle_tree.changelog_index() as u64;

    let mut compressed_account_to_nullify = Vec::new();
    println!("\n --------------------------------------------------\n\t\t NULLIFYING LEAVES\n --------------------------------------------------");
    for (i, element) in nullifier_queue.iter() {
        if element.sequence_number().is_none() {
            println!("element to nullify: {:?}", element.value_bytes());
            let leaf_index: usize = merkle_tree.get_leaf_index(&element.value_bytes()).unwrap();
            println!("leaf_index: {:?}", leaf_index);
            compressed_account_to_nullify.push((i, element.value_bytes()));
        }
    }

    for (index_in_nullifier_queue, compressed_account) in compressed_account_to_nullify.iter() {
        let leaf_index: usize = merkle_tree.get_leaf_index(compressed_account).unwrap();
        let proof: Vec<[u8; 32]> = merkle_tree
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
                &state_merkle_tree_accounts.merkle_tree,
                &state_merkle_tree_accounts.nullifier_queue,
            ),
        ];

        create_and_send_transaction(context, &instructions, &payer.pubkey(), &[&payer])
            .await
            .unwrap();

        let nullifier_queue = unsafe {
            get_hash_set::<
                u16,
                account_compression::initialize_nullifier_queue::NullifierQueueAccount,
            >(context, state_merkle_tree_accounts.nullifier_queue)
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
            state_merkle_tree_accounts.merkle_tree,
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
    // Locally nullify all leaves
    for (_, compressed_account) in compressed_account_to_nullify.iter() {
        let leaf_index = merkle_tree.get_leaf_index(compressed_account).unwrap();
        merkle_tree.update(&[0u8; 32], leaf_index).unwrap();
    }
    let merkle_tree_account = AccountZeroCopy::<StateMerkleTreeAccount>::new(
        context,
        state_merkle_tree_accounts.merkle_tree,
    )
    .await;
    let onchain_merkle_tree = merkle_tree_account
        .deserialized()
        .copy_merkle_tree()
        .unwrap();
    assert_eq!(onchain_merkle_tree.root().unwrap(), merkle_tree.root());
}
