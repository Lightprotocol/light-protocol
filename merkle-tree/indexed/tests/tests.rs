use std::cell::{RefCell, RefMut};

use ark_ff::{BigInteger, BigInteger256};
use light_bounded_vec::BoundedVec;
use light_concurrent_merkle_tree::light_hasher::{Hasher, Poseidon};
use light_indexed_merkle_tree::{
    array::{IndexingArray, IndexingElement},
    errors::IndexedMerkleTreeError,
    reference, IndexedMerkleTree,
};
use light_utils::bigint::be_bytes_to_bigint;
use thiserror::Error;

const MERKLE_TREE_HEIGHT: usize = 4;
const MERKLE_TREE_CHANGELOG: usize = 256;
const MERKLE_TREE_ROOTS: usize = 1024;
const MERKLE_TREE_CANOPY: usize = 0;

const QUEUE_ELEMENTS: usize = 1024;

const INDEXING_ARRAY_ELEMENTS: usize = 1024;

const NR_NULLIFIERS: usize = 2;

/// A mock function which imitates a Merkle tree program instruction for
/// inserting nullifiers into the queue.
fn program_insert<H>(
    // PDA
    mut queue: RefMut<'_, IndexingArray<H, u16, BigInteger256, QUEUE_ELEMENTS>>,
    // Instruction data
    nullifiers: [[u8; 32]; NR_NULLIFIERS],
) -> Result<(), IndexedMerkleTreeError>
where
    H: Hasher,
{
    for i in 0..NR_NULLIFIERS {
        let nullifier = be_bytes_to_bigint(&nullifiers[i]).unwrap();
        queue.append(nullifier)?;
    }
    Ok(())
}

#[derive(Error, Debug)]
enum RelayerUpdateError {
    #[error("Updating Merkle tree failed, {0:?}")]
    MerkleTreeUpdate(Vec<IndexedMerkleTreeError>),
}

/// A mock function which imitates a Merkle tree program instruction for
/// inserting nullifiers from the queue to the tree.
fn program_update<H>(
    // PDAs
    queue: &mut RefMut<'_, IndexingArray<H, u16, BigInteger256, QUEUE_ELEMENTS>>,
    merkle_tree: &mut RefMut<'_, IndexedMerkleTree<H, usize, BigInteger256, MERKLE_TREE_HEIGHT>>,
    // Instruction data
    changelog_index: u16,
    queue_index: u16,
    nullifier_index: usize,
    nullifier_next_index: usize,
    nullifier_next_value: BigInteger256,
    low_nullifier: IndexingElement<usize, BigInteger256>,
    low_nullifier_next_value: BigInteger256,
    low_nullifier_proof: &mut BoundedVec<[u8; 32]>,
) -> Result<(), IndexedMerkleTreeError>
where
    H: Hasher,
{
    // Remove the nullifier from the queue.
    let nullifier = queue.dequeue_at(queue_index).unwrap().unwrap();

    // Update the nullifier with ranges adjusted to the Merkle tree state,
    // coming from relayer.
    let nullifier: IndexingElement<usize, BigInteger256> = IndexingElement {
        index: nullifier_index,
        value: nullifier.value,
        next_index: nullifier_next_index,
    };

    // Update the Merkle tree.
    merkle_tree.update(
        usize::from(changelog_index),
        nullifier,
        nullifier_next_value,
        low_nullifier,
        low_nullifier_next_value,
        low_nullifier_proof,
    )
}

/// A mock function which imitates a relayer endpoint for updating the
/// nullifier Merkle tree.
fn relayer_update<H>(
    // PDAs
    queue: &mut RefMut<'_, IndexingArray<H, u16, BigInteger256, QUEUE_ELEMENTS>>,
    merkle_tree: &mut RefMut<'_, IndexedMerkleTree<H, usize, BigInteger256, MERKLE_TREE_HEIGHT>>,
) -> Result<(), RelayerUpdateError>
where
    H: Hasher,
{
    let mut relayer_indexing_array =
        IndexingArray::<H, usize, BigInteger256, INDEXING_ARRAY_ELEMENTS>::default();
    let mut relayer_merkle_tree = reference::IndexedMerkleTree::<H, usize, BigInteger256>::new(
        MERKLE_TREE_HEIGHT,
        MERKLE_TREE_ROOTS,
        MERKLE_TREE_CANOPY,
    )
    .unwrap();

    let mut update_errors: Vec<IndexedMerkleTreeError> = Vec::new();

    while !queue.is_empty() {
        let changelog_index = merkle_tree.changelog_index();

        let lowest_from_queue = match queue.lowest() {
            Some(lowest) => lowest,
            None => break,
        };

        // Create new element from the dequeued value.
        let (old_low_nullifier, old_low_nullifier_next_value) = relayer_indexing_array
            .find_low_element(&lowest_from_queue.value)
            .unwrap();
        let nullifier_bundle = relayer_indexing_array
            .new_element_with_low_element_index(old_low_nullifier.index, lowest_from_queue.value)
            .unwrap();
        let mut low_nullifier_proof = relayer_merkle_tree
            .get_proof_of_leaf(usize::from(old_low_nullifier.index))
            .unwrap();

        // Update on-chain tree.
        let update_successful = match program_update(
            queue,
            merkle_tree,
            changelog_index as u16,
            lowest_from_queue.index,
            nullifier_bundle.new_element.index,
            nullifier_bundle.new_element.next_index,
            nullifier_bundle.new_element_next_value,
            old_low_nullifier,
            old_low_nullifier_next_value,
            &mut low_nullifier_proof,
        ) {
            Ok(_) => true,
            Err(e) => {
                update_errors.push(e);
                false
            }
        };

        // Check if the on-chain Merkle tree was really updated.
        if update_successful {
            // Update off-chain tree.
            relayer_merkle_tree
                .update(
                    &nullifier_bundle.new_low_element,
                    &nullifier_bundle.new_element,
                    &nullifier_bundle.new_element_next_value,
                )
                .unwrap();

            let low_nullifier_leaf = nullifier_bundle
                .new_low_element
                .hash::<H>(&nullifier_bundle.new_element.value)
                .unwrap();
            let low_nullifier_proof = relayer_merkle_tree
                .get_proof_of_leaf(nullifier_bundle.new_low_element.index())
                .unwrap();
            merkle_tree
                .validate_proof(
                    &low_nullifier_leaf,
                    nullifier_bundle.new_low_element.index(),
                    &low_nullifier_proof,
                )
                .unwrap();

            let new_nullifier_leaf = nullifier_bundle
                .new_element
                .hash::<H>(&nullifier_bundle.new_element_next_value)
                .unwrap();
            let new_nullifier_proof = relayer_merkle_tree
                .get_proof_of_leaf(nullifier_bundle.new_element.index())
                .unwrap();
            merkle_tree
                .validate_proof(
                    &new_nullifier_leaf,
                    nullifier_bundle.new_element.index(),
                    &new_nullifier_proof,
                )
                .unwrap();

            // Insert the element to the indexing array.
            relayer_indexing_array
                .append_with_low_element_index(
                    nullifier_bundle.new_low_element.index,
                    nullifier_bundle.new_element.value,
                )
                .unwrap();
        }
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
    let onchain_queue: RefCell<IndexingArray<H, u16, BigInteger256, QUEUE_ELEMENTS>> =
        RefCell::new(IndexingArray::default());
    let onchain_tree: RefCell<IndexedMerkleTree<H, usize, BigInteger256, MERKLE_TREE_HEIGHT>> =
        RefCell::new(IndexedMerkleTree::new(
            MERKLE_TREE_HEIGHT,
            MERKLE_TREE_CHANGELOG,
            MERKLE_TREE_ROOTS,
            MERKLE_TREE_CANOPY,
        ));
    onchain_tree.borrow_mut().init().unwrap();

    // Insert a pair of nullifiers.
    let nullifier1 = BigInteger256::from(30_u32);
    let nullifier2 = BigInteger256::from(10_u32);
    program_insert::<H>(
        onchain_queue.borrow_mut(),
        [
            nullifier1.to_bytes_be().try_into().unwrap(),
            nullifier2.to_bytes_be().try_into().unwrap(),
        ],
    )
    .unwrap();

    // Insert an another pair of nullifiers.
    let nullifier3 = BigInteger256::from(20_u32);
    let nullifier4 = BigInteger256::from(50_u32);
    program_insert::<H>(
        onchain_queue.borrow_mut(),
        [
            nullifier3.to_bytes_be().try_into().unwrap(),
            nullifier4.to_bytes_be().try_into().unwrap(),
        ],
    )
    .unwrap();

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
    let onchain_queue: RefCell<IndexingArray<H, u16, BigInteger256, QUEUE_ELEMENTS>> =
        RefCell::new(IndexingArray::default());
    let onchain_tree: RefCell<IndexedMerkleTree<H, usize, BigInteger256, MERKLE_TREE_HEIGHT>> =
        RefCell::new(IndexedMerkleTree::new(
            MERKLE_TREE_HEIGHT,
            MERKLE_TREE_CHANGELOG,
            MERKLE_TREE_ROOTS,
            MERKLE_TREE_CANOPY,
        ));
    onchain_tree.borrow_mut().init().unwrap();

    // Insert a pair of nulifiers.
    let nullifier1: [u8; 32] = BigInteger256::from(30_u32)
        .to_bytes_be()
        .try_into()
        .unwrap();
    let nullifier2: [u8; 32] = BigInteger256::from(10_u32)
        .to_bytes_be()
        .try_into()
        .unwrap();
    program_insert::<H>(onchain_queue.borrow_mut(), [nullifier1, nullifier2]).unwrap();

    // Try inserting the same pair into the queue. It should fail with an error.
    assert!(matches!(
        program_insert::<H>(onchain_queue.borrow_mut(), [nullifier1, nullifier2]),
        Err(IndexedMerkleTreeError::LowElementGreaterOrEqualToNewElement),
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
    let nullifier3 = BigInteger256::from(25_u32);
    let nullifier4 = BigInteger256::from(5_u32);
    program_insert::<H>(
        onchain_queue.borrow_mut(),
        [
            nullifier3.to_bytes_be().try_into().unwrap(),
            nullifier4.to_bytes_be().try_into().unwrap(),
        ],
    )
    .unwrap();
    // We expect exactly two errors (for the invalid nullifiers). No more, no
    // less.
    let res = relayer_update::<H>(
        &mut onchain_queue.borrow_mut(),
        &mut onchain_tree.borrow_mut(),
    );
    assert!(matches!(res, Err(RelayerUpdateError::MerkleTreeUpdate(_))));
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
    let onchain_queue: RefCell<IndexingArray<H, u16, BigInteger256, QUEUE_ELEMENTS>> =
        RefCell::new(IndexingArray::default());
    let onchain_tree: RefCell<IndexedMerkleTree<H, usize, BigInteger256, MERKLE_TREE_HEIGHT>> =
        RefCell::new(IndexedMerkleTree::new(
            MERKLE_TREE_HEIGHT,
            MERKLE_TREE_CHANGELOG,
            MERKLE_TREE_ROOTS,
            MERKLE_TREE_CANOPY,
        ));
    onchain_tree.borrow_mut().init().unwrap();

    // Local artifacts.
    let mut local_indexing_array =
        IndexingArray::<H, usize, BigInteger256, INDEXING_ARRAY_ELEMENTS>::default();
    let mut local_merkle_tree = reference::IndexedMerkleTree::<H, usize, BigInteger256>::new(
        MERKLE_TREE_HEIGHT,
        MERKLE_TREE_ROOTS,
        MERKLE_TREE_CANOPY,
    )
    .unwrap();

    // Insert a pair of nullifiers, correctly. Just do it with relayer.
    let nullifier1 = BigInteger256::from(30_u32);
    let nullifier2 = BigInteger256::from(10_u32);
    onchain_queue.borrow_mut().append(nullifier1).unwrap();
    onchain_queue.borrow_mut().append(nullifier2).unwrap();
    let nullifier_bundle = local_indexing_array.append(nullifier1).unwrap();
    local_merkle_tree
        .update(
            &nullifier_bundle.new_low_element,
            &nullifier_bundle.new_element,
            &nullifier_bundle.new_element_next_value,
        )
        .unwrap();
    let nullifier_bundle = local_indexing_array.append(nullifier2).unwrap();
    local_merkle_tree
        .update(
            &nullifier_bundle.new_low_element,
            &nullifier_bundle.new_element,
            &nullifier_bundle.new_element_next_value,
        )
        .unwrap();
    relayer_update(
        &mut onchain_queue.borrow_mut(),
        &mut onchain_tree.borrow_mut(),
    )
    .unwrap();

    // Try inserting nullifier 20, while pointing to index 1 (value 30) as low
    // nullifier. Point to index 2 (value 10) as next value.
    // Therefore, the new element is lowe than the supposed low element.
    let nullifier3 = BigInteger256::from(20_u32);
    onchain_queue.borrow_mut().append(nullifier3).unwrap();
    let changelog_index = onchain_tree.borrow_mut().changelog_index();
    // Index of our new nullifier in the queue.
    let queue_index = 1_u16;
    // Index of our new nullifier in the tree / on-chain state.
    let nullifier_index = 3_usize;
    // (Invalid) index of the next nullifier.
    let nullifier_next_index = 2_usize;
    // (Invalid) value of the next nullifier.
    let nullifier_next_value = nullifier2;
    // (Invalid) low nullifier.
    let low_nullifier = local_indexing_array.get(1).cloned().unwrap();
    let low_nullifier_next_value = local_indexing_array
        .get(usize::from(low_nullifier.next_index))
        .cloned()
        .unwrap()
        .value;
    let mut low_nullifier_proof = local_merkle_tree.get_proof_of_leaf(1).unwrap();
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
            low_nullifier_next_value,
            &mut low_nullifier_proof,
        ),
        Err(IndexedMerkleTreeError::LowElementGreaterOrEqualToNewElement)
    ));

    // Try inserting nullifier 50, while pointing to index 0 as low nullifier.
    // Therefore, the new element is greate than next element.
    let nullifier3 = BigInteger256::from(50_u32);
    onchain_queue.borrow_mut().append(nullifier3).unwrap();
    let changelog_index = onchain_tree.borrow_mut().changelog_index();
    // Index of our new nullifier in the queue.
    let queue_index = 1_u16;
    // Index of our new nullifier in the tree / on-chain state.
    let nullifier_index = 3_usize;
    // (Invalid) index of the next nullifier. Value: 30.
    let nullifier_next_index = 1_usize;
    // (Invalid) value of the next nullifier.
    let nullifier_next_value = nullifier1;
    // (Invalid) low nullifier.
    let low_nullifier = local_indexing_array.get(0).cloned().unwrap();
    let low_nullifier_next_value = local_indexing_array
        .get(usize::from(low_nullifier.next_index))
        .cloned()
        .unwrap()
        .value;
    let mut low_nullifier_proof = local_merkle_tree.get_proof_of_leaf(0).unwrap();
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
            low_nullifier_next_value,
            &mut low_nullifier_proof,
        ),
        Err(IndexedMerkleTreeError::NewElementGreaterOrEqualToNextElement)
    ));
}

#[test]
pub fn test_insert_invalid_low_element_poseidon() {
    insert_invalid_low_element::<Poseidon>()
}
