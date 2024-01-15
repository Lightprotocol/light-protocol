use std::cell::{RefCell, RefMut};

use light_hasher::{errors::HasherError, Hasher, Poseidon};
use light_indexed_merkle_tree::{
    array::{IndexingArray, IndexingElement},
    reference, IndexedMerkleTree,
};
use thiserror::Error;

const MERKLE_TREE_HEIGHT: usize = 4;
const MERKLE_TREE_CHANGELOG: usize = 256;
const MERKLE_TREE_ROOTS: usize = 1024;

const QUEUE_ELEMENTS: usize = 1024;

const INDEXING_ARRAY_ELEMENTS: usize = 1024;

const NR_NULLIFIERS: usize = 2;

/// A mock function which imitates a Merkle tree program instruction for
/// inserting nullifiers into the queue.
fn program_insert<H>(
    // PDA
    mut queue: RefMut<'_, IndexingArray<H, QUEUE_ELEMENTS>>,
    // Instruction data
    nullifiers: [[u8; 32]; NR_NULLIFIERS],
) -> Result<(), HasherError>
where
    H: Hasher,
{
    for i in 0..NR_NULLIFIERS {
        queue.append(nullifiers[i])?;
    }
    Ok(())
}

#[derive(Error, Debug)]
enum RelayerUpdateError {
    #[error("Updating Merkle tree failed, {0:?}")]
    MerkleTreeUpdate(Vec<HasherError>),
}

/// A mock function which imitates a Merkle tree program instruction for
/// inserting nullifiers from the queue to the tree.
fn program_update<H>(
    // PDAs
    queue: &mut RefMut<'_, IndexingArray<H, QUEUE_ELEMENTS>>,
    merkle_tree: &mut RefMut<
        '_,
        IndexedMerkleTree<H, MERKLE_TREE_HEIGHT, MERKLE_TREE_CHANGELOG, MERKLE_TREE_ROOTS>,
    >,
    // Instruction data
    changelog_index: u16,
    queue_index: u16,
    nullifier_index: u16,
    nullifier_next_index: u16,
    nullifier_next_value: [u8; 32],
    low_nullifier: IndexingElement,
    low_nullifier_proof: [[u8; 32]; MERKLE_TREE_HEIGHT],
) -> Result<(), HasherError>
where
    H: Hasher,
{
    // Remove the nullifier from the queue.
    let mut nullifier = queue.dequeue_at(queue_index as usize).unwrap().unwrap();

    // Update the nullifier with ranges adjusted to the Merkle tree state,
    // coming from relayer.
    nullifier.index = nullifier_index as usize;
    nullifier.next_index = nullifier_next_index as usize;
    nullifier.next_value = nullifier_next_value;

    // Update the Merkle tree.
    merkle_tree.update(
        changelog_index as usize,
        nullifier,
        low_nullifier,
        &low_nullifier_proof,
    )
}

/// A mock function which imitates a relayer endpoint for updating the
/// nullifier Merkle tree.
fn relayer_update<H>(
    // PDAs
    queue: &mut RefMut<'_, IndexingArray<H, QUEUE_ELEMENTS>>,
    merkle_tree: &mut RefMut<
        '_,
        IndexedMerkleTree<H, MERKLE_TREE_HEIGHT, MERKLE_TREE_CHANGELOG, MERKLE_TREE_ROOTS>,
    >,
) -> Result<(), RelayerUpdateError>
where
    H: Hasher,
{
    let mut relayer_indexing_array = IndexingArray::<H, INDEXING_ARRAY_ELEMENTS>::default();
    let mut relayer_merkle_tree =
        reference::IndexedMerkleTree::<H, MERKLE_TREE_HEIGHT, MERKLE_TREE_ROOTS>::new().unwrap();

    let mut update_errors: Vec<HasherError> = Vec::new();

    while !queue.is_empty() {
        let changelog_index = merkle_tree.changelog_index();

        let lowest_from_queue = match queue.lowest() {
            Some(lowest) => lowest,
            None => break,
        };

        // Create new element from the dequeued value.
        let old_low_nullifier = relayer_indexing_array.find_low_element(&lowest_from_queue.value);
        let (new_low_nullifier, nullifier) = relayer_indexing_array
            .new_element_with_low_element_index(old_low_nullifier.index, lowest_from_queue.value);
        let low_nullifier_proof = relayer_merkle_tree.get_proof_of_leaf(old_low_nullifier.index);

        // Update on-chain tree.
        if let Err(e) = program_update(
            queue,
            merkle_tree,
            changelog_index as u16,
            lowest_from_queue.index as u16,
            nullifier.index as u16,
            nullifier.next_index as u16,
            nullifier.next_value,
            old_low_nullifier,
            low_nullifier_proof,
        ) {
            update_errors.push(e);
        }

        // Update off-chain tree.
        relayer_merkle_tree
            .update(new_low_nullifier, nullifier)
            .unwrap();

        // Insert the element to the indexing array.
        relayer_indexing_array
            .append_with_low_element_index(new_low_nullifier.index, nullifier.value)
            .unwrap();
    }

    if update_errors.is_empty() {
        Ok(())
    } else {
        Err(RelayerUpdateError::MerkleTreeUpdate(update_errors))
    }
}

/// Tests the valid case of:
///
/// * Inserting nullifiers to the queue.
/// * Calling the relayer to update the on-chain nullifier Merkle tree.
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

    // Insert a pair of nullifiers.
    let nullifier1: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 30,
    ];
    let nullifier2: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 10,
    ];
    program_insert::<H>(onchain_queue.borrow_mut(), [nullifier1, nullifier2]).unwrap();

    // Insert an another pair of nullifiers.
    let nullifier3: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 20,
    ];
    let nullifier4: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 50,
    ];
    program_insert::<H>(onchain_queue.borrow_mut(), [nullifier3, nullifier4]).unwrap();

    // Call relayer to update the tree.
    relayer_update::<H>(
        &mut onchain_queue.borrow_mut(),
        &mut onchain_tree.borrow_mut(),
    )
    .unwrap();
}

#[test]
pub fn test_insert_and_update_poseidon() {
    insert_and_update::<Poseidon>()
}

/// Tests the invalid case of inserting the same nullifiers multiple times into
/// the queue and Merkle tree - an attempt of double spending.
fn double_spend<H>()
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

    // Insert a pair of nulifiers.
    let nullifier1: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 30,
    ];
    let nullifier2: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 10,
    ];
    program_insert::<H>(onchain_queue.borrow_mut(), [nullifier1, nullifier2]).unwrap();

    // Try inserting the same pair into the queue. It should fail with an error.
    assert!(matches!(
        program_insert::<H>(onchain_queue.borrow_mut(), [nullifier1, nullifier2]),
        Err(HasherError::LowElementGreaterOrEqualToNewElement),
    ));

    // Update the on-chain tree (so it contains the nullifiers we inserted).
    relayer_update::<H>(
        &mut onchain_queue.borrow_mut(),
        &mut onchain_tree.borrow_mut(),
    )
    .unwrap();

    // The nullifiers are in the tree and not in the queue anymore. We can try
    // our luck with double-spending again.
    program_insert::<H>(onchain_queue.borrow_mut(), [nullifier1, nullifier2]).unwrap();
    // At the same time, insert also some new nullifiers which aren't spent
    // yet. We want to make sure that they will be processed successfully and
    // only the invalid nullifiers will produce errors.
    let nullifier3: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 25,
    ];
    let nullifier4: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 5,
    ];
    program_insert::<H>(onchain_queue.borrow_mut(), [nullifier3, nullifier4]).unwrap();
    // We expect exactly two errors (for the invalid nullifiers). No more, no
    // less.
    let _expected_err: Result<(), RelayerUpdateError> =
        Err(RelayerUpdateError::MerkleTreeUpdate(vec![
            HasherError::InvalidProof,
            HasherError::InvalidProof,
        ]));
    assert!(matches!(
        relayer_update::<H>(
            &mut onchain_queue.borrow_mut(),
            &mut onchain_tree.borrow_mut(),
        ),
        _expected_err,
    ));
}

#[test]
pub fn test_double_spend_queue_poseidon() {
    double_spend::<Poseidon>()
}

/// Try to insert a nullifier to the tree while pointing to an invalid low
/// nullifier.
///
/// Such invalid insertion needs to be performed manually, without relayer's
/// help (which would always insert that nullifier correctly).
fn insert_invalid_low_element<H>()
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

    // Local artifacts.
    let mut local_indexing_array = IndexingArray::<H, INDEXING_ARRAY_ELEMENTS>::default();
    let mut local_merkle_tree =
        reference::IndexedMerkleTree::<H, MERKLE_TREE_HEIGHT, MERKLE_TREE_ROOTS>::new().unwrap();

    // Insert a pair of nullifiers, correctly. Just do it with relayer.
    let nullifier1: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 30,
    ];
    let nullifier2: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 10,
    ];
    onchain_queue.borrow_mut().append(nullifier1).unwrap();
    onchain_queue.borrow_mut().append(nullifier2).unwrap();
    let (new_low_nullifier, new_nullifier) = local_indexing_array.append(nullifier1).unwrap();
    local_merkle_tree
        .update(new_low_nullifier, new_nullifier)
        .unwrap();
    let (new_low_nullifier, new_nullifier) = local_indexing_array.append(nullifier2).unwrap();
    local_merkle_tree
        .update(new_low_nullifier, new_nullifier)
        .unwrap();
    relayer_update(
        &mut onchain_queue.borrow_mut(),
        &mut onchain_tree.borrow_mut(),
    )
    .unwrap();

    // Try inserting nullifier 20, while pointing to index 1 (value 30) as low
    // nullifier. Point to index 2 (value 10) as next value.
    // Therefore, the new element is lowe than the supposed low element.
    let nullifier3: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 20,
    ];
    onchain_queue.borrow_mut().append(nullifier3).unwrap();
    let changelog_index = onchain_tree.borrow_mut().changelog_index();
    // Index of our new nullifier in the queue.
    let queue_index = 1_u16;
    // Index of our new nullifier in the tree / on-chain state.
    let nullifier_index = 3_u16;
    // (Invalid) index of the next nullifier.
    let nullifier_next_index = 2_u16;
    // (Invalid) value of the next nullifier.
    let nullifier_next_value = nullifier2;
    // (Invalid) low nullifier.
    let low_nullifier = local_indexing_array.get(1).cloned().unwrap();
    let low_nullifier_proof = local_merkle_tree.get_proof_of_leaf(1);
    assert!(matches!(
        program_update(
            &mut onchain_queue.borrow_mut(),
            &mut onchain_tree.borrow_mut(),
            changelog_index as u16,
            queue_index,
            nullifier_index,
            nullifier_next_index,
            nullifier_next_value,
            low_nullifier,
            low_nullifier_proof,
        ),
        Err(HasherError::LowElementGreaterOrEqualToNewElement)
    ));

    // Try inserting nullifier 50, while pointing to index 0 as low nullifier.
    // Therefore, the new element is greate than next element.
    let nullifier3: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 50,
    ];
    onchain_queue.borrow_mut().append(nullifier3).unwrap();
    let changelog_index = onchain_tree.borrow_mut().changelog_index();
    // Index of our new nullifier in the queue.
    let queue_index = 1_u16;
    // Index of our new nullifier in the tree / on-chain state.
    let nullifier_index = 3_u16;
    // (Invalid) index of the next nullifier. Value: 30.
    let nullifier_next_index = 1_u16;
    // (Invalid) value of the next nullifier.
    let nullifier_next_value = nullifier1;
    // (Invalid) low nullifier.
    let low_nullifier = local_indexing_array.get(0).cloned().unwrap();
    let low_nullifier_proof = local_merkle_tree.get_proof_of_leaf(0);
    assert!(matches!(
        program_update(
            &mut onchain_queue.borrow_mut(),
            &mut onchain_tree.borrow_mut(),
            changelog_index as u16,
            queue_index,
            nullifier_index,
            nullifier_next_index,
            nullifier_next_value,
            low_nullifier,
            low_nullifier_proof,
        ),
        Err(HasherError::NewElementGreaterOrEqualToNextElement)
    ));
}

#[test]
pub fn test_insert_invalid_low_element_poseidon() {
    insert_invalid_low_element::<Poseidon>()
}
