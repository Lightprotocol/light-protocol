use std::cell::{RefCell, RefMut};

use borsh::{BorshDeserialize, BorshSerialize};
use light_bounded_vec::BoundedVec;
use light_concurrent_merkle_tree::{
    changelog,
    errors::ConcurrentMerkleTreeError,
    light_hasher::{Hasher, Poseidon},
    ConcurrentMerkleTree,
};
use light_indexed_merkle_tree::{
    array::{IndexedArray, IndexedElement},
    errors::IndexedMerkleTreeError,
    reference, IndexedMerkleTree, FIELD_SIZE_SUB_ONE,
};
use light_utils::bigint::bigint_to_be_bytes_array;
use num_bigint::{BigUint, ToBigUint};
use num_traits::{FromBytes, Num};
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
    queue_index: u16,
    nullifier_index: usize,
    nullifier_next_index: usize,
    low_nullifier: IndexedElement<usize>,
    low_nullifier_next_value: &BigUint,
    low_nullifier_proof: &mut BoundedVec<[u8; 32]>,
) -> Result<(), IndexedMerkleTreeError>
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
        nullifier,
        low_nullifier,
        low_nullifier_next_value,
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

        let lowest_from_queue = match queue.lowest() {
            Some(lowest) => lowest,
            None => break,
        };

        // Create new element from the dequeued value.
        let (old_low_nullifier, old_low_nullifier_next_value) = relayer_indexing_array
            .find_low_element(&lowest_from_queue.value)
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
            lowest_from_queue.index,
            nullifier_bundle.new_element.index,
            nullifier_bundle.new_element.next_index,
            old_low_nullifier,
            &old_low_nullifier_next_value,
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
    let changelog_index = onchain_tree.borrow_mut().changelog_index();
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
    let changelog_index = onchain_tree.borrow_mut().changelog_index();
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
            queue_index,
            nullifier_index,
            nullifier_next_index,
            low_nullifier,
            &low_nullifier_next_value,
            &mut low_nullifier_proof,
        ),
        Err(IndexedMerkleTreeError::NewElementGreaterOrEqualToNextElement)
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

#[derive(Debug, Default, Clone, Copy, BorshSerialize, BorshDeserialize)]
pub struct RawIndexedElement {
    pub value: [u8; 32],
    pub next_index: usize,
    pub next_value: [u8; 32],
    pub index: usize,
    pub next_value_index: usize,
    pub next_value_next_value: [u8; 32],
    pub next_value_path: [[u8; 32]; 10],
}

/// Check whether low element has changed
/// - possibilities:
///   1. check Merkle proof of -> too expensive costs like 20k CU
///   2. check whether the array contains the index of the low element if yes then check the next value
///   3. insert the index in to hash set that stores key: index value: next_value, can be overwritten if capacity requires,
///         the hashset needs to be able to hold the same element multiple times though, does it?
///         maybe not, maybe we can just say that we allow every element once and when we use it then we overwrite it immediately instead of overwriting an expired element
///  
pub fn patch_low_element<const CHANGELOG_CAPACITY: usize>(
    value: &BigUint,
    low_element: &IndexedElement<usize>,
    changelog_array: &[RawIndexedElement; CHANGELOG_CAPACITY],
) -> Option<(IndexedElement<usize>, [u8; 32], Option<[[u8; 32]; 10]>)> {
    // TODO: bench how find performs vs filter
    // filter would have the advantage that it allows for multiple changes in between
    // although we just need most recent, then we can zero out the value that we read
    let changelog_element_index = changelog_array
        .iter()
        .position(|element| element.index == low_element.index);

    match changelog_element_index {
        Some(changelog_element_index) => {
            let changelog_element = &changelog_array[changelog_element_index];
            println!("changelog_element: {:?}", changelog_element);
            println!(
                "hangelog_element.next_value: {:?}",
                BigUint::from_bytes_be(&changelog_element.next_value)
            );
            println!("low_element.value: {:?}", low_element.value);
            let mut next_value_path = None;
            // check that the changelog element is still the low element
            // if not take the next element which is the high element of that low element
            let new_low_element = if BigUint::from_bytes_be(&changelog_element.next_value) < *value
            {
                next_value_path = Some(changelog_element.next_value_path);
                IndexedElement {
                    value: BigUint::from_bytes_be(&changelog_element.next_value),
                    index: changelog_element.next_index,
                    next_index: changelog_element.next_value_index,
                }
            } else {
                IndexedElement {
                    value: BigUint::from_bytes_be(&changelog_element.value),
                    index: changelog_element.index,
                    next_index: changelog_element.next_index,
                }
            };
            println!("changelog_element: {:?}", changelog_element);

            // Only use changelog event values, since these originate from an account -> can be trusted
            // let new_low_element =
            Some((
                new_low_element,
                changelog_element.next_value,
                next_value_path,
            ))
        }
        None => None,
    }
}

/**
 *
 * Range Hash (value, next_index, next_value) -> need next value not next value index
 * Update of a range:
 * 1. Find the low element, low element points to the next hight element
 * 2. update low element with H (low_value, new_inserted_value_index, new_inserted_value)
 * 3. append the tree with H(new_inserted_value,index_of_next_value, next_value)
 *
 */
const merkle_tree_height: usize = 10;
const canopy_depth: usize = 0;
/// This test is generating a situation where the low element has to be patched.
/// Scenario:
/// 1. two parties start with the initialized indexing array
/// 2. both parties compute their values with the empty indexed Merkle tree state
/// 3. party one inserts first
/// 4. party two needs to patch the low element because the low element has changed
/// 5. party two inserts
#[test]
pub fn functional_changelog_test() {
    let address_1 = 30_u32.to_biguint().unwrap();
    let address_2 = 10_u32.to_biguint().unwrap();

    // perform_change_log_test(address_1.clone(), address_2.clone());

    let address_1 = 10_u32.to_biguint().unwrap();
    let address_2 = 30_u32.to_biguint().unwrap();
    perform_change_log_test(address_1.clone(), address_2.clone());
}

fn perform_change_log_test(address_1: BigUint, address_2: BigUint) {
    let mut relayer_indexing_array =
        IndexedArray::<Poseidon, usize, INDEXING_ARRAY_ELEMENTS>::default();
    relayer_indexing_array.init().unwrap();
    // appends the first element
    let mut relayer_merkle_tree =
        reference::IndexedMerkleTree::<Poseidon, usize>::new(merkle_tree_height, canopy_depth)
            .unwrap();

    let mut onchain_changelog_array = [RawIndexedElement::default(); 256];
    let mut next_changelog_index = 0;
    let mut onchain_indexed_merkle_tree =
        IndexedMerkleTree::<Poseidon, usize, merkle_tree_height>::new(
            merkle_tree_height,
            MERKLE_TREE_CHANGELOG,
            MERKLE_TREE_ROOTS,
            canopy_depth,
        )
        .unwrap();
    onchain_indexed_merkle_tree.init().unwrap();
    let init_value = BigUint::from_str_radix(FIELD_SIZE_SUB_ONE, 10).unwrap();
    IndexedMerkleTree::initialize_address_merkle_tree(
        &mut onchain_indexed_merkle_tree,
        init_value.clone(),
    )
    .unwrap();
    relayer_merkle_tree.init().unwrap();
    assert_eq!(
        relayer_merkle_tree.root(),
        onchain_indexed_merkle_tree.root().unwrap(),
        "environment setup failed relayer and onchain indexed Merkle tree roots are inconsistent"
    );
    let mut actor_1_indexed_array_state = relayer_indexing_array.clone();
    let mut actor_2_indexed_array_state = relayer_indexing_array.clone();

    let (old_low_address_1, old_low_address_next_value_1) = actor_1_indexed_array_state
        .find_low_element(&address_1)
        .unwrap();
    let address_bundle_1 = relayer_indexing_array
        .new_element_with_low_element_index(old_low_address_1.index, &address_1)
        .unwrap();
    let change_log_index = onchain_indexed_merkle_tree.changelog_index();
    {
        let mut low_element_proof_1 = relayer_merkle_tree
            .get_proof_of_leaf(old_low_address_1.index, false)
            .unwrap();

        onchain_indexed_merkle_tree
            .update(
                change_log_index.clone(),
                address_bundle_1.new_element.clone(),
                old_low_address_1.clone(),
                &old_low_address_next_value_1,
                &mut low_element_proof_1,
            )
            .unwrap();
        onchain_changelog_array[next_changelog_index] = RawIndexedElement {
            value: bigint_to_be_bytes_array::<32>(&address_bundle_1.new_low_element.value).unwrap(),
            next_index: address_bundle_1.new_low_element.index,
            next_value: bigint_to_be_bytes_array::<32>(&address_bundle_1.new_element.value)
                .unwrap(),
            index: old_low_address_1.index,
            next_value_index: address_bundle_1.new_element.index,
            next_value_next_value: bigint_to_be_bytes_array::<32>(&old_low_address_next_value_1)
                .unwrap(),
            next_value_path: onchain_indexed_merkle_tree
                .merkle_tree
                .filled_subtrees
                .to_array()
                .unwrap(),
        };
        // next_changelog_index += 1;
        // onchain_changelog_array[next_changelog_index] = RawIndexedElement {
        //     value: bigint_to_be_bytes_array::<32>(&address_bundle_1.new_element.value).unwrap(),
        //     next_index: address_bundle_1.new_element.index,
        //     next_value: bigint_to_be_bytes_array::<32>(&old_low_address_next_value_1).unwrap(),
        //     index: old_low_address_1.index,
        // };
        next_changelog_index += 1;
    }

    // getting parameters for the second actor with the pre update state
    let (mut old_low_address, mut old_low_address_next_value) = actor_2_indexed_array_state
        .find_low_element(&address_1)
        .unwrap();
    let address_bundle = relayer_indexing_array
        .new_element_with_low_element_index(old_low_address.index, &address_2)
        .unwrap();

    let mut low_element_proof = relayer_merkle_tree
        .get_proof_of_leaf(old_low_address.index, false)
        .unwrap();

    println!("old_low_address: {:?}", old_low_address);
    println!(
        "old_low_address_next_value: {:?}",
        old_low_address_next_value
    );
    // patching the low element
    let res = patch_low_element(
        &address_bundle.new_element.value,
        &old_low_address,
        &onchain_changelog_array,
    );
    let updated_path: Option<[[u8; 32]; 10]> = match res {
        Some((patched_low_element, patched_low_element_next_value, path_option)) => {
            old_low_address = patched_low_element;
            old_low_address_next_value = BigUint::from_bytes_be(&patched_low_element_next_value);
            path_option
        }
        None => None,
    };
    if updated_path.is_some() {
        println!("patched proof");
        low_element_proof = BoundedVec::from_array(&updated_path.unwrap());
    }
    println!("patched old_low_address: {:?}", old_low_address);
    println!(
        "patched  old_low_address_next_value: {:?}",
        old_low_address_next_value
    );
    // how do I proof that my first low element was included in the tree at some point?
    // - it doesn't matter because we are just going to use the onchain values and they can be trusted they have been checked before
    // -> discard all the value that are inputs
    // so I patched the proof
    onchain_indexed_merkle_tree
        .update(
            change_log_index,
            address_bundle.new_element,
            old_low_address,
            &old_low_address_next_value,
            &mut low_element_proof,
            // &mut onchain_changelog_array,
        )
        .unwrap();

    // update the relayer state
    relayer_merkle_tree
        .append(&address_1, &mut relayer_indexing_array)
        .unwrap();

    relayer_merkle_tree
        .append(&address_2, &mut relayer_indexing_array)
        .unwrap();
}
