use std::cell::{RefCell, RefMut};

use light_hasher::{Hasher, Poseidon};
use light_indexed_merkle_tree::{
    array::{IndexingArray, IndexingElement},
    reference, IndexedMerkleTree,
};

const MERKLE_TREE_HEIGHT: usize = 4;
const MERKLE_TREE_CHANGELOG: usize = 256;
const MERKLE_TREE_ROOTS: usize = 1024;

const QUEUE_ELEMENTS: usize = 1024;

const INDEXING_ARRAY_ELEMENTS: usize = 1024;

const NR_NULLIFIERS: usize = 2;

fn insert_and_update<H>()
where
    H: Hasher,
{
    // On-chain PDAs.
    let onchain_queue: RefCell<IndexingArray<H, QUEUE_ELEMENTS>> =
        RefCell::new(IndexingArray::default());
    let onchain_tree: RefCell<
        IndexedMerkleTree<H, MERKLE_TREE_HEIGHT, MERKLE_TREE_CHANGELOG, MERKLE_TREE_ROOTS>,
    > = RefCell::new(IndexedMerkleTree::default());

    onchain_tree.borrow_mut().init().unwrap();

    // Client.
    let mut local_indexing_array = IndexingArray::<H, INDEXING_ARRAY_ELEMENTS>::default();

    // Pairs of nullifiers we want to insert.
    let nullifier1: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 30,
    ];
    let nullifier2: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 10,
    ];

    local_indexing_array.append(nullifier1).unwrap();
    local_indexing_array.append(nullifier2).unwrap();

    cpi_insert::<H>(onchain_queue.borrow_mut(), [nullifier1, nullifier2]);

    let nullifier3: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 20,
    ];
    let nullifier4: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 50,
    ];

    local_indexing_array.append(nullifier3).unwrap();
    local_indexing_array.append(nullifier4).unwrap();

    cpi_insert::<H>(onchain_queue.borrow_mut(), [nullifier3, nullifier4]);

    // Call relayer to update the tree.
    relayer_update::<H>(
        &mut onchain_queue.borrow_mut(),
        &mut onchain_tree.borrow_mut(),
    );
}

fn cpi_insert<H>(
    // PDA
    mut queue: RefMut<'_, IndexingArray<H, QUEUE_ELEMENTS>>,
    // Instruction data
    nullifiers: [[u8; 32]; NR_NULLIFIERS],
) where
    H: Hasher,
{
    for i in 0..NR_NULLIFIERS {
        queue.append(nullifiers[i]).unwrap();
    }
}

fn relayer_update<H>(
    // PDAs
    queue: &mut RefMut<'_, IndexingArray<H, QUEUE_ELEMENTS>>,
    merkle_tree: &mut RefMut<
        '_,
        IndexedMerkleTree<H, MERKLE_TREE_HEIGHT, MERKLE_TREE_CHANGELOG, MERKLE_TREE_ROOTS>,
    >,
) where
    H: Hasher,
{
    let mut relayer_indexing_array = IndexingArray::<H, INDEXING_ARRAY_ELEMENTS>::default();
    let mut relayer_merkle_tree =
        reference::IndexedMerkleTree::<H, MERKLE_TREE_HEIGHT, MERKLE_TREE_ROOTS>::new().unwrap();

    while !queue.empty() {
        let changelog_index = merkle_tree.changelog_index();

        let lowest_from_queue = match queue.lowest() {
            Some(lowest) => lowest,
            None => break,
        };

        // Create new element from the dequeued value.
        let low_nullifier_index = relayer_indexing_array
            .find_low_element_index(&lowest_from_queue.value)
            .unwrap();
        let (new_low_nullifier, nullifier) = relayer_indexing_array
            .new_element_with_low_element_index(low_nullifier_index, lowest_from_queue.value);
        let low_nullifier_proof = relayer_merkle_tree.get_proof_of_leaf(low_nullifier_index);

        let old_low_nullifier_leaf = relayer_merkle_tree.node(low_nullifier_index);

        // Update on-chain tree.
        cpi_update(
            queue,
            merkle_tree,
            changelog_index as u16,
            lowest_from_queue.index as u16,
            nullifier,
            old_low_nullifier_leaf,
            new_low_nullifier,
            low_nullifier_index as u16,
            low_nullifier_proof,
        );

        // Update off-chain tree.
        relayer_merkle_tree
            .update(
                new_low_nullifier,
                new_low_nullifier.index,
                nullifier,
                nullifier.index,
            )
            .unwrap();

        // Insert the element to the indexing array.
        relayer_indexing_array
            .append_with_low_element_index(new_low_nullifier.index, nullifier.value);
    }
}

fn cpi_update<H>(
    // PDAs
    queue: &mut RefMut<'_, IndexingArray<H, QUEUE_ELEMENTS>>,
    merkle_tree: &mut RefMut<
        '_,
        IndexedMerkleTree<H, MERKLE_TREE_HEIGHT, MERKLE_TREE_CHANGELOG, MERKLE_TREE_ROOTS>,
    >,
    // Instruction data
    changelog_index: u16,
    queue_index: u16,
    new_nullifier: IndexingElement,
    old_low_nullifier_leaf: [u8; 32],
    new_low_nullifier: IndexingElement,
    low_nullifier_index: u16,
    low_nullifier_proof: [[u8; 32]; MERKLE_TREE_HEIGHT],
) where
    H: Hasher,
{
    // Remove the nullifier from the queue.
    queue.dequeue_at(queue_index as usize).unwrap().unwrap();

    // Update the Merkle tree.
    merkle_tree
        .update(
            changelog_index as usize,
            new_nullifier,
            &old_low_nullifier_leaf,
            new_low_nullifier,
            low_nullifier_index as usize,
            &low_nullifier_proof,
        )
        .unwrap();
}

#[test]
pub fn test_insert_and_update() {
    insert_and_update::<Poseidon>()
}
