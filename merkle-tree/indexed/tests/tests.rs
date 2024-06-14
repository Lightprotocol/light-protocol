use std::cell::{RefCell, RefMut};

use light_bounded_vec::BoundedVec;
use light_concurrent_merkle_tree::{
    event::IndexedMerkleTreeUpdate,
    light_hasher::{Hasher, Poseidon},
};
use light_indexed_merkle_tree::{
    array::{IndexedArray, IndexedElement},
    errors::IndexedMerkleTreeError,
    reference, IndexedMerkleTree,
};
use light_utils::bigint::bigint_to_be_bytes_array;
use num_bigint::{BigUint, ToBigUint};
use num_traits::FromBytes;
use thiserror::Error;

const MERKLE_TREE_HEIGHT: usize = 4;
const MERKLE_TREE_CHANGELOG: usize = 256;
const MERKLE_TREE_ROOTS: usize = 1024;
const MERKLE_TREE_CANOPY: usize = 0;
const MERKLE_TREE_INDEXED_CHANGELOG: usize = 64;

const QUEUE_ELEMENTS: usize = 1024;

const INDEXING_ARRAY_ELEMENTS: usize = 1024;

const NR_NULLIFIERS: usize = 2;

/// A mock function which imitates a Merkle tree program instruction for
/// inserting nullifiers into the queue.
fn program_insert<H>(
    // PDA
    mut queue: RefMut<'_, IndexedArray<H, u16, QUEUE_ELEMENTS>>,
    // Instruction data
    nullifiers: [[u8; 32]; NR_NULLIFIERS],
) -> Result<(), IndexedMerkleTreeError>
where
    H: Hasher,
{
    for i in 0..NR_NULLIFIERS {
        let nullifier = BigUint::from_be_bytes(nullifiers[i].as_slice());
        queue.append(&nullifier)?;
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
    queue: &mut RefMut<'_, IndexedArray<H, u16, QUEUE_ELEMENTS>>,
    merkle_tree: &mut RefMut<'_, IndexedMerkleTree<H, usize, MERKLE_TREE_HEIGHT>>,
    // Instruction data
    changelog_index: u16,
    indexed_changelog_index: u16,
    queue_index: u16,
    nullifier_index: usize,
    nullifier_next_index: usize,
    low_nullifier: IndexedElement<usize>,
    low_nullifier_next_value: &BigUint,
    low_nullifier_proof: &mut BoundedVec<[u8; 32]>,
) -> Result<IndexedMerkleTreeUpdate<usize>, IndexedMerkleTreeError>
where
    H: Hasher,
{
    // Remove the nullifier from the queue.
    let nullifier = queue.dequeue_at(queue_index).unwrap().unwrap();

    // Update the nullifier with ranges adjusted to the Merkle tree state,
    // coming from relayer.
    let nullifier: IndexedElement<usize> = IndexedElement {
        index: nullifier_index,
        value: nullifier.value,
        next_index: nullifier_next_index,
    };

    // Update the Merkle tree.
    merkle_tree.update(
        usize::from(changelog_index),
        usize::from(indexed_changelog_index),
        nullifier,
        low_nullifier.clone(),
        low_nullifier_next_value.clone(),
        low_nullifier_proof,
    )
}

// TODO: unify these helpers with MockIndexer
/// A mock function which imitates a relayer endpoint for updating the
/// nullifier Merkle tree.
fn relayer_update<H>(
    // PDAs
    queue: &mut RefMut<'_, IndexedArray<H, u16, QUEUE_ELEMENTS>>,
    merkle_tree: &mut RefMut<'_, IndexedMerkleTree<H, usize, MERKLE_TREE_HEIGHT>>,
) -> Result<(), RelayerUpdateError>
where
    H: Hasher,
{
    let mut relayer_indexing_array = IndexedArray::<H, usize, INDEXING_ARRAY_ELEMENTS>::default();
    let mut relayer_merkle_tree =
        reference::IndexedMerkleTree::<H, usize>::new(MERKLE_TREE_HEIGHT, MERKLE_TREE_CANOPY)
            .unwrap();

    let mut update_errors: Vec<IndexedMerkleTreeError> = Vec::new();

    while !queue.is_empty() {
        let changelog_index = merkle_tree.changelog_index();
        let indexed_changelog_index = merkle_tree.indexed_changelog_index();

        let lowest_from_queue = match queue.lowest() {
            Some(lowest) => lowest,
            None => break,
        };

        // Create new element from the dequeued value.
        let (old_low_nullifier, old_low_nullifier_next_value) = relayer_indexing_array
            .find_low_element_for_nonexistent(&lowest_from_queue.value)
            .unwrap();
        let nullifier_bundle = relayer_indexing_array
            .new_element_with_low_element_index(old_low_nullifier.index, &lowest_from_queue.value)
            .unwrap();
        let mut low_nullifier_proof = relayer_merkle_tree
            .get_proof_of_leaf(usize::from(old_low_nullifier.index), false)
            .unwrap();

        // Update on-chain tree.
        let update_successful = match program_update(
            queue,
            merkle_tree,
            changelog_index as u16,
            indexed_changelog_index as u16,
            lowest_from_queue.index,
            nullifier_bundle.new_element.index,
            nullifier_bundle.new_element.next_index,
            old_low_nullifier,
            &old_low_nullifier_next_value,
            &mut low_nullifier_proof,
        ) {
            Ok(event) => {
                assert_eq!(
                    event.new_low_element.index,
                    nullifier_bundle.new_low_element.index
                );
                assert_eq!(
                    event.new_low_element.next_index,
                    nullifier_bundle.new_low_element.next_index
                );
                assert_eq!(
                    event.new_low_element.value,
                    bigint_to_be_bytes_array::<32>(&nullifier_bundle.new_low_element.value)
                        .unwrap()
                );
                assert_eq!(
                    event.new_low_element.next_value,
                    bigint_to_be_bytes_array::<32>(&nullifier_bundle.new_element.value).unwrap()
                );
                let leaf_hash = nullifier_bundle
                    .new_low_element
                    .hash::<H>(&nullifier_bundle.new_element.value)
                    .unwrap();
                assert_eq!(event.new_low_element_hash, leaf_hash);
                let leaf_hash = nullifier_bundle
                    .new_element
                    .hash::<H>(&nullifier_bundle.new_element_next_value)
                    .unwrap();
                assert_eq!(event.new_high_element_hash, leaf_hash);
                assert_eq!(
                    event.new_high_element.index,
                    nullifier_bundle.new_element.index
                );
                assert_eq!(
                    event.new_high_element.next_index,
                    nullifier_bundle.new_element.next_index
                );
                assert_eq!(
                    event.new_high_element.value,
                    bigint_to_be_bytes_array::<32>(&nullifier_bundle.new_element.value).unwrap()
                );
                assert_eq!(
                    event.new_high_element.next_value,
                    bigint_to_be_bytes_array::<32>(&nullifier_bundle.new_element_next_value)
                        .unwrap()
                );
                true
            }
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
                .get_proof_of_leaf(nullifier_bundle.new_low_element.index(), false)
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
                .get_proof_of_leaf(nullifier_bundle.new_element.index(), false)
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
                    &nullifier_bundle.new_element.value,
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
    let onchain_queue: RefCell<IndexedArray<H, u16, QUEUE_ELEMENTS>> =
        RefCell::new(IndexedArray::default());
    let onchain_tree: RefCell<IndexedMerkleTree<H, usize, MERKLE_TREE_HEIGHT>> = RefCell::new(
        IndexedMerkleTree::new(
            MERKLE_TREE_HEIGHT,
            MERKLE_TREE_CHANGELOG,
            MERKLE_TREE_ROOTS,
            MERKLE_TREE_CANOPY,
            1,
        )
        .unwrap(),
    );
    onchain_tree.borrow_mut().init().unwrap();

    // Insert a pair of nullifiers.
    let nullifier1 = 30_u32.to_biguint().unwrap();
    let nullifier2 = 10_u32.to_biguint().unwrap();
    program_insert::<H>(
        onchain_queue.borrow_mut(),
        [
            bigint_to_be_bytes_array(&nullifier1).unwrap(),
            bigint_to_be_bytes_array(&nullifier2).unwrap(),
        ],
    )
    .unwrap();

    // Insert an another pair of nullifiers.
    let nullifier3 = 20_u32.to_biguint().unwrap();
    let nullifier4 = 50_u32.to_biguint().unwrap();
    program_insert::<H>(
        onchain_queue.borrow_mut(),
        [
            bigint_to_be_bytes_array(&nullifier3).unwrap(),
            bigint_to_be_bytes_array(&nullifier4).unwrap(),
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
    let onchain_queue: RefCell<IndexedArray<H, u16, QUEUE_ELEMENTS>> =
        RefCell::new(IndexedArray::default());
    let onchain_tree: RefCell<IndexedMerkleTree<H, usize, MERKLE_TREE_HEIGHT>> = RefCell::new(
        IndexedMerkleTree::new(
            MERKLE_TREE_HEIGHT,
            MERKLE_TREE_CHANGELOG,
            MERKLE_TREE_ROOTS,
            MERKLE_TREE_CANOPY,
            1,
        )
        .unwrap(),
    );
    onchain_tree.borrow_mut().init().unwrap();

    // Insert a pair of nulifiers.
    let nullifier1 = 30_u32.to_biguint().unwrap();
    let nullifier1: [u8; 32] = bigint_to_be_bytes_array(&nullifier1).unwrap();
    let nullifier2 = 10_u32.to_biguint().unwrap();
    let nullifier2: [u8; 32] = bigint_to_be_bytes_array(&nullifier2).unwrap();
    program_insert::<H>(onchain_queue.borrow_mut(), [nullifier1, nullifier2]).unwrap();

    // Try inserting the same pair into the queue. It should fail with an error.
    let res = program_insert::<H>(onchain_queue.borrow_mut(), [nullifier1, nullifier2]);
    assert!(matches!(
        res,
        Err(IndexedMerkleTreeError::ElementAlreadyExists),
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
    let nullifier3 = 25_u32.to_biguint().unwrap();
    let nullifier4 = 5_u32.to_biguint().unwrap();
    program_insert::<H>(
        onchain_queue.borrow_mut(),
        [
            bigint_to_be_bytes_array(&nullifier3).unwrap(),
            bigint_to_be_bytes_array(&nullifier4).unwrap(),
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
    let onchain_queue: RefCell<IndexedArray<H, u16, QUEUE_ELEMENTS>> =
        RefCell::new(IndexedArray::default());
    let onchain_tree: RefCell<IndexedMerkleTree<H, usize, MERKLE_TREE_HEIGHT>> = RefCell::new(
        IndexedMerkleTree::new(
            MERKLE_TREE_HEIGHT,
            MERKLE_TREE_CHANGELOG,
            MERKLE_TREE_ROOTS,
            MERKLE_TREE_CANOPY,
            1,
        )
        .unwrap(),
    );
    onchain_tree.borrow_mut().init().unwrap();

    // Local artifacts.
    let mut local_indexed_array = IndexedArray::<H, usize, INDEXING_ARRAY_ELEMENTS>::default();
    let mut local_merkle_tree =
        reference::IndexedMerkleTree::<H, usize>::new(MERKLE_TREE_HEIGHT, MERKLE_TREE_CANOPY)
            .unwrap();

    // Insert a pair of nullifiers, correctly. Just do it with relayer.
    let nullifier1 = 30_u32.to_biguint().unwrap();
    let nullifier2 = 10_u32.to_biguint().unwrap();
    onchain_queue.borrow_mut().append(&nullifier1).unwrap();
    onchain_queue.borrow_mut().append(&nullifier2).unwrap();
    let nullifier_bundle = local_indexed_array.append(&nullifier1).unwrap();
    local_merkle_tree
        .update(
            &nullifier_bundle.new_low_element,
            &nullifier_bundle.new_element,
            &nullifier_bundle.new_element_next_value,
        )
        .unwrap();
    let nullifier_bundle = local_indexed_array.append(&nullifier2).unwrap();
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
    let nullifier3 = 20_u32.to_biguint().unwrap();
    onchain_queue.borrow_mut().append(&nullifier3).unwrap();
    let changelog_index = onchain_tree.borrow().changelog_index();
    let indexed_changelog_index = onchain_tree.borrow().indexed_changelog_index();
    // Index of our new nullifier in the queue.
    let queue_index = 1_u16;
    // Index of our new nullifier in the tree / on-chain state.
    let nullifier_index = 3_usize;
    // (Invalid) index of the next nullifier.
    let nullifier_next_index = 2_usize;
    // (Invalid) low nullifier.
    let low_nullifier = local_indexed_array.get(1).cloned().unwrap();
    let low_nullifier_next_value = local_indexed_array
        .get(usize::from(low_nullifier.next_index))
        .cloned()
        .unwrap()
        .value;
    let mut low_nullifier_proof = local_merkle_tree.get_proof_of_leaf(1, false).unwrap();
    assert!(matches!(
        program_update(
            &mut onchain_queue.borrow_mut(),
            &mut onchain_tree.borrow_mut(),
            changelog_index as u16,
            indexed_changelog_index as u16,
            queue_index,
            nullifier_index,
            nullifier_next_index,
            low_nullifier,
            &low_nullifier_next_value,
            &mut low_nullifier_proof,
        ),
        Err(IndexedMerkleTreeError::LowElementGreaterOrEqualToNewElement)
    ));

    // Try inserting nullifier 50, while pointing to index 0 as low nullifier.
    // Therefore, the new element is greate than next element.
    let nullifier3 = 50_u32.to_biguint().unwrap();
    onchain_queue.borrow_mut().append(&nullifier3).unwrap();
    let changelog_index = onchain_tree.borrow().changelog_index();
    let indexed_changelog_index = onchain_tree.borrow().indexed_changelog_index();
    // Index of our new nullifier in the queue.
    let queue_index = 1_u16;
    // Index of our new nullifier in the tree / on-chain state.
    let nullifier_index = 3_usize;
    // (Invalid) index of the next nullifier. Value: 30.
    let nullifier_next_index = 1_usize;
    // (Invalid) low nullifier.
    let low_nullifier = local_indexed_array.get(0).cloned().unwrap();
    let low_nullifier_next_value = local_indexed_array
        .get(usize::from(low_nullifier.next_index))
        .cloned()
        .unwrap()
        .value;
    let mut low_nullifier_proof = local_merkle_tree.get_proof_of_leaf(0, false).unwrap();
    assert!(matches!(
        program_update(
            &mut onchain_queue.borrow_mut(),
            &mut onchain_tree.borrow_mut(),
            changelog_index as u16,
            indexed_changelog_index as u16,
            queue_index,
            nullifier_index,
            nullifier_next_index,
            low_nullifier,
            &low_nullifier_next_value,
            &mut low_nullifier_proof,
        ),
        Err(IndexedMerkleTreeError::NewElementGreaterOrEqualToNextElement)
    ));
    let nullifier4 = 45_u32.to_biguint().unwrap();
    onchain_queue.borrow_mut().append(&nullifier3).unwrap();
    let changelog_index = onchain_tree.borrow().changelog_index();
    let indexed_changelog_index = onchain_tree.borrow().indexed_changelog_index();
    let (low_nullifier, low_nullifier_next_value) = local_indexed_array
        .find_low_element_for_nonexistent(&nullifier4)
        .unwrap();
    let nullifier_next_index = low_nullifier.next_index + 1;
    let mut low_nullifier_proof = local_merkle_tree
        .get_proof_of_leaf(low_nullifier.index(), false)
        .unwrap();
    let result = program_update(
        &mut onchain_queue.borrow_mut(),
        &mut onchain_tree.borrow_mut(),
        changelog_index as u16,
        indexed_changelog_index as u16,
        queue_index,
        nullifier_index,
        nullifier_next_index,
        low_nullifier,
        &low_nullifier_next_value,
        &mut low_nullifier_proof,
    );
    println!("result {:?}", result);
    assert!(matches!(
        result,
        Err(IndexedMerkleTreeError::NewElementNextIndexMismatch)
    ));
}

#[test]
pub fn test_insert_invalid_low_element_poseidon() {
    insert_invalid_low_element::<Poseidon>()
}

#[test]
pub fn functional_non_inclusion_test() {
    let mut relayer_indexing_array =
        IndexedArray::<Poseidon, usize, INDEXING_ARRAY_ELEMENTS>::default();

    // appends the first element
    let mut relayer_merkle_tree = reference::IndexedMerkleTree::<Poseidon, usize>::new(
        MERKLE_TREE_HEIGHT,
        MERKLE_TREE_CANOPY,
    )
    .unwrap();
    let nullifier1 = 30_u32.to_biguint().unwrap();
    relayer_merkle_tree
        .append(&nullifier1, &mut relayer_indexing_array)
        .unwrap();
    // indexed array:
    // element: 0
    // value: 0
    // next_value: 30
    // index: 0
    // element: 1
    // value: 30
    // next_value: 0
    // index: 1
    // merkle tree:
    // leaf index: 0 = H(0, 1, 30) //Hash(value, next_index, next_value)
    // leaf index: 1 = H(30, 0, 0)
    let indexed_array_element_0 = relayer_indexing_array.get(0).unwrap();
    assert_eq!(indexed_array_element_0.value, 0_u32.to_biguint().unwrap());
    assert_eq!(indexed_array_element_0.next_index, 1);
    assert_eq!(indexed_array_element_0.index, 0);
    let indexed_array_element_1 = relayer_indexing_array.get(1).unwrap();
    assert_eq!(indexed_array_element_1.value, 30_u32.to_biguint().unwrap());
    assert_eq!(indexed_array_element_1.next_index, 0);
    assert_eq!(indexed_array_element_1.index, 1);

    let leaf_0 = relayer_merkle_tree.merkle_tree.leaf(0);
    let leaf_1 = relayer_merkle_tree.merkle_tree.leaf(1);
    assert_eq!(
        leaf_0,
        Poseidon::hashv(&[
            &0_u32.to_biguint().unwrap().to_bytes_be(),
            &1_u32.to_biguint().unwrap().to_bytes_be(),
            &30_u32.to_biguint().unwrap().to_bytes_be()
        ])
        .unwrap()
    );
    assert_eq!(
        leaf_1,
        Poseidon::hashv(&[
            &30_u32.to_biguint().unwrap().to_bytes_be(),
            &0_u32.to_biguint().unwrap().to_bytes_be(),
            &0_u32.to_biguint().unwrap().to_bytes_be()
        ])
        .unwrap()
    );

    let non_inclusion_proof = relayer_merkle_tree
        .get_non_inclusion_proof(&10_u32.to_biguint().unwrap(), &relayer_indexing_array)
        .unwrap();
    assert_eq!(non_inclusion_proof.root, relayer_merkle_tree.root());
    assert_eq!(
        non_inclusion_proof.value,
        bigint_to_be_bytes_array::<32>(&10_u32.to_biguint().unwrap()).unwrap()
    );
    assert_eq!(non_inclusion_proof.leaf_lower_range_value, [0; 32]);
    assert_eq!(
        non_inclusion_proof.leaf_higher_range_value,
        bigint_to_be_bytes_array::<32>(&30_u32.to_biguint().unwrap()).unwrap()
    );
    assert_eq!(non_inclusion_proof.leaf_index, 0);

    relayer_merkle_tree
        .verify_non_inclusion_proof(&non_inclusion_proof)
        .unwrap();
}

// /**
//  *
//  * Range Hash (value, next_index, next_value) -> need next value not next value index
//  * Update of a range:
//  * 1. Find the low element, low element points to the next hight element
//  * 2. update low element with H (low_value, new_inserted_value_index, new_inserted_value)
//  * 3. append the tree with H(new_inserted_value,index_of_next_value, next_value)
//  *
//  */
// /// This test is generating a situation where the low element has to be patched.
// /// Scenario:
// /// 1. two parties start with the initialized indexing array
// /// 2. both parties compute their values with the empty indexed Merkle tree state
// /// 3. party one inserts first
// /// 4. party two needs to patch the low element because the low element has changed
// /// 5. party two inserts
// Commented because the test is not working
// TODO: figure out address Merkle tree changelog
// #[test]
// pub fn functional_changelog_test() {
//     let address_1 = 30_u32.to_biguint().unwrap();
//     let address_2 = 10_u32.to_biguint().unwrap();
// cargo test -- --nocapture print_test_data
// #[test]
// pub fn print_test_data() {
//     let mut relayer_indexing_array =
//         IndexedArray::<Poseidon, usize, INDEXING_ARRAY_ELEMENTS>::default();
//     relayer_indexing_array.init().unwrap();
//     let mut relayer_merkle_tree =
//         reference::IndexedMerkleTree::<Poseidon, usize>::new(26, 10).unwrap();
//     relayer_merkle_tree.init().unwrap();
//     println!("indexed mt inited root {:?}", relayer_merkle_tree.root());
//     let address1 = 30_u32.to_biguint().unwrap();

//     let test_address: BigUint = BigUint::from_bytes_be(&[
//         171, 159, 63, 33, 62, 94, 156, 27, 61, 216, 203, 164, 91, 229, 110, 16, 230, 124, 129, 133,
//         222, 159, 99, 235, 50, 181, 94, 203, 105, 23, 82,
//     ]);

//     let non_inclusion_proof_0 = relayer_merkle_tree
//         .get_non_inclusion_proof(&test_address, &relayer_indexing_array)
//         .unwrap();

//     println!("non inclusion proof init {:?}", non_inclusion_proof_0);

//     relayer_merkle_tree
//         .append(&address1, &mut relayer_indexing_array)
//         .unwrap();

//     println!(
//         "indexed mt with one append {:?}",
//         relayer_merkle_tree.root()
//     );
//     println!(
//         "indexed array state element 0 {:?}",
//         relayer_indexing_array.get(0).unwrap()
//     );
//     println!(
//         "indexed array state element 1 {:?}",
//         relayer_indexing_array.get(1).unwrap()
//     );
//     println!(
//         "indexed array state element 2 {:?}",
//         relayer_indexing_array.get(2).unwrap()
//     );

//     let address2 = 42_u32.to_biguint().unwrap();
//     relayer_merkle_tree
//         .append(&address2, &mut relayer_indexing_array)
//         .unwrap();

//     println!(
//         "indexed mt with two appends {:?}",
//         relayer_merkle_tree.root()
//     );
//     let non_inclusion_proof = relayer_merkle_tree
//         .get_non_inclusion_proof(&address2, &relayer_indexing_array)
//         .unwrap();
//     println!("non inclusion proof address 2 {:?}", non_inclusion_proof);
//     println!(
//         "indexed array state element 0 {:?}",
//         relayer_indexing_array.get(0).unwrap()
//     );
//     println!(
//         "indexed array state element 1 {:?}",
//         relayer_indexing_array.get(1).unwrap()
//     );
//     println!(
//         "indexed array state element 2 {:?}",
//         relayer_indexing_array.get(2).unwrap()
//     );
//     println!(
//         "indexed array state element 3 {:?}",
//         relayer_indexing_array.get(3).unwrap()
//     );

//     let address3 = 12_u32.to_biguint().unwrap();

//     let non_inclusion_proof = relayer_merkle_tree
//     .get_non_inclusion_proof(&address3, &relayer_indexing_array)
//     .unwrap();

//     relayer_merkle_tree
//         .append(&address3, &mut relayer_indexing_array)
//         .unwrap();

//     println!(
//         "indexed mt with three appends {:?}",
//         relayer_merkle_tree.root()
//     );

//     println!("non inclusion proof address 3 {:?}", non_inclusion_proof);
//     println!(
//         "indexed array state element 0 {:?}",
//         relayer_indexing_array.get(0).unwrap()
//     );
//     println!(
//         "indexed array state element 1 {:?}",
//         relayer_indexing_array.get(1).unwrap()
//     );
//     println!(
//         "indexed array state element 2 {:?}",
//         relayer_indexing_array.get(2).unwrap()
//     );
//     println!(
//         "indexed array state element 3 {:?}",
//         relayer_indexing_array.get(3).unwrap()
//     );
//     println!(
//         "indexed array state element 4 {:?}",
//         relayer_indexing_array.get(4).unwrap()
//     );

//     // // indexed array:
//     // // element: 0
//     // // value: 0
//     // // next_value: 30
//     // // index: 0
//     // // element: 1
//     // // value: 30
//     // // next_value: 0
//     // // index: 1
//     // // merkle tree:
//     // // leaf index: 0 = H(0, 1, 30) //Hash(value, next_index, next_value)
//     // // leaf index: 1 = H(30, 0, 0)
//     // let indexed_array_element_0 = relayer_indexing_array.get(0).unwrap();
//     // assert_eq!(indexed_array_element_0.value, 0_u32.to_biguint().unwrap());
//     // assert_eq!(indexed_array_element_0.next_index, 1);
//     // assert_eq!(indexed_array_element_0.index, 0);
//     // let indexed_array_element_1 = relayer_indexing_array.get(1).unwrap();
//     // assert_eq!(indexed_array_element_1.value, 30_u32.to_biguint().unwrap());
//     // assert_eq!(indexed_array_element_1.next_index, 0);
//     // assert_eq!(indexed_array_element_1.index, 1);

//     // let leaf_0 = relayer_merkle_tree.merkle_tree.leaf(0);
//     // let leaf_1 = relayer_merkle_tree.merkle_tree.leaf(1);
//     // assert_eq!(
//     //     leaf_0,
//     //     Poseidon::hashv(&[
//     //         &0_u32.to_biguint().unwrap().to_bytes_be(),
//     //         &1_u32.to_biguint().unwrap().to_bytes_be(),
//     //         &30_u32.to_biguint().unwrap().to_bytes_be()
//     //     ])
//     //     .unwrap()
//     // );
//     // assert_eq!(
//     //     leaf_1,
//     //     Poseidon::hashv(&[
//     //         &30_u32.to_biguint().unwrap().to_bytes_be(),
//     //         &0_u32.to_biguint().unwrap().to_bytes_be(),
//     //         &0_u32.to_biguint().unwrap().to_bytes_be()
//     //     ])
//     //     .unwrap()
//     // );

//     // let non_inclusion_proof = relayer_merkle_tree
//     //     .get_non_inclusion_proof(&10_u32.to_biguint().unwrap(), &relayer_indexing_array)
//     //     .unwrap();
//     // assert_eq!(non_inclusion_proof.root, relayer_merkle_tree.root());
//     // assert_eq!(
//     //     non_inclusion_proof.value,
//     //     bigint_to_be_bytes_array::<32>(&10_u32.to_biguint().unwrap()).unwrap()
//     // );
//     // assert_eq!(non_inclusion_proof.leaf_lower_range_value, [0; 32]);
//     // assert_eq!(
//     //     non_inclusion_proof.leaf_higher_range_value,
//     //     bigint_to_be_bytes_array::<32>(&30_u32.to_biguint().unwrap()).unwrap()
//     // );
//     // assert_eq!(non_inclusion_proof.leaf_index, 0);

//     // relayer_merkle_tree
//     //     .verify_non_inclusion_proof(&non_inclusion_proof)
//     //     .unwrap();
// }

/**
 *
 * Range Hash (value, next_index, next_value) -> need next value not next value index
 * Update of a range:
 * 1. Find the low element, low element points to the next hight element
 * 2. update low element with H (low_value, new_inserted_value_index, new_inserted_value)
 * 3. append the tree with H(new_inserted_value,index_of_next_value, next_value)
 *
*/
// This test is generating a situation where the low element has to be patched.
// Scenario:
// 1. two parties start with the initialized indexing array
// 2. both parties compute their values with the empty indexed Merkle tree state
// 3. party one inserts first
// 4. party two needs to patch the low element because the low element has changed
// 5. party two inserts
#[test]
fn functional_changelog_test_1() {
    let address_1 = 30_u32.to_biguint().unwrap();
    let address_2 = 10_u32.to_biguint().unwrap();

    perform_change_log_test(&[address_1, address_2]);
}

/// Performs conflicting Merkle tree update operations where multiple actors
/// try to add new ranges when using the same (for the most of actors - outdated)
/// Merkle proofs and changelog indices.
fn perform_change_log_test(addresses: &[BigUint]) {
    // Initialize the trees and indexed array.
    let mut relayer_indexed_array =
        IndexedArray::<Poseidon, usize, INDEXING_ARRAY_ELEMENTS>::default();
    relayer_indexed_array.init().unwrap();
    let mut relayer_merkle_tree =
        reference::IndexedMerkleTree::<Poseidon, usize>::new(10, 0).unwrap();
    let mut onchain_indexed_merkle_tree = IndexedMerkleTree::<Poseidon, usize, 10>::new(
        10,
        MERKLE_TREE_CHANGELOG,
        MERKLE_TREE_ROOTS,
        0,
        MERKLE_TREE_INDEXED_CHANGELOG,
    )
    .unwrap();
    onchain_indexed_merkle_tree.init().unwrap();
    onchain_indexed_merkle_tree.add_highest_element().unwrap();
    relayer_merkle_tree.init().unwrap();
    assert_eq!(
        relayer_merkle_tree.root(),
        onchain_indexed_merkle_tree.root(),
        "environment setup failed relayer and onchain indexed Merkle tree roots are inconsistent"
    );

    // Perform updates for each actor, where every of them is using the same
    // changelog indices, generating a conflict which needs to be solved by
    // patching from changelog.
    let mut indexed_arrays = vec![relayer_indexed_array.clone(); addresses.len()];
    let changelog_index = onchain_indexed_merkle_tree.changelog_index();
    let indexed_changelog_index = onchain_indexed_merkle_tree.indexed_changelog_index();
    for (address, indexed_array) in addresses.iter().zip(indexed_arrays.iter_mut()) {
        let (old_low_address, old_low_address_next_value) = indexed_array
            .find_low_element_for_nonexistent(&address)
            .unwrap();
        let address_bundle = indexed_array
            .new_element_with_low_element_index(old_low_address.index, address)
            .unwrap();

        let mut low_element_proof = relayer_merkle_tree
            .get_proof_of_leaf(old_low_address.index, false)
            .unwrap();

        onchain_indexed_merkle_tree
            .update(
                changelog_index,
                indexed_changelog_index,
                address_bundle.new_element,
                old_low_address,
                old_low_address_next_value,
                &mut low_element_proof,
            )
            .unwrap();
    }
}
