use std::str::FromStr;

use light_hasher::{bigint::bigint_to_be_bytes_array, Poseidon};
use light_indexed_array::{
    array::{IndexedArray, IndexedElement},
    changelog::RawIndexedElement,
    HIGHEST_ADDRESS_PLUS_ONE,
};
use light_merkle_tree_reference::indexed::IndexedMerkleTree;
use light_sparse_merkle_tree::indexed_changelog::{
    patch_indexed_changelogs, IndexedChangelogEntry,
};
use num_bigint::BigUint;
use num_traits::FromPrimitive;
use rand::{
    prelude::SliceRandom,
    rngs::{StdRng, ThreadRng},
    Rng, SeedableRng,
};

/// Performs conflicting Merkle tree updates where multiple actors try to add
/// add new ranges when using the same (for the most of actors - outdated)
/// Merkle proofs and changelog indices.
///
/// Scenario:
///
/// 1. Two paries start with the same indexed array state.
/// 2. Both parties compute their values with the same indexed Merkle tree
///    state.
/// 3. Party one inserts first.
/// 4. Party two needs to patch the low element, because the low element has
///    changed.
/// 5. Party two inserts.
/// 6. Party N needs to patch the low element, because the low element has
///    changed.
/// 7. Party N inserts.
///
/// `DOUBLE_SPEND` indicates whether the provided addresses are an attempt to
/// double-spend by the subsequent parties. When set to `true`, we expect
/// subsequent updates to fail.
#[test]
fn test_indexed_changelog() {
    let seed = ThreadRng::default().gen();
    println!("seed {:?}", seed);
    let rng = &mut StdRng::from_seed(seed);
    for _ in 0..10 {
        let mut indexed_merkle_tree = IndexedMerkleTree::<Poseidon, usize>::new(8, 0).unwrap();
        let mut man_indexed_array = IndexedArray::<Poseidon, usize>::new(
            BigUint::from_usize(0).unwrap(),
            BigUint::from_str(HIGHEST_ADDRESS_PLUS_ONE).unwrap(),
        );
        let mut addresses = vec![];
        for i in 2..100 {
            let address = BigUint::from_usize(i).unwrap();
            addresses.push(address);
        }
        addresses.shuffle(rng);

        let next_index = indexed_merkle_tree.merkle_tree.rightmost_index;
        let mut indexed_changelog: Vec<IndexedChangelogEntry<usize, 8>> = Vec::new();
        let mut low_element_values = Vec::new();
        let mut low_element_indices = Vec::new();
        let mut low_element_next_indices = Vec::new();
        let mut low_element_next_values = Vec::new();
        let mut low_element_proofs: Vec<Vec<[u8; 32]>> = Vec::new();
        // get inputs
        for address in addresses.iter() {
            let non_inclusion_proof = indexed_merkle_tree
                .get_non_inclusion_proof(address)
                .unwrap();
            low_element_values.push(non_inclusion_proof.leaf_lower_range_value);
            low_element_indices.push(non_inclusion_proof.leaf_index);
            low_element_next_indices.push(non_inclusion_proof.next_index);
            low_element_next_values.push(non_inclusion_proof.leaf_higher_range_value);

            low_element_proofs.push(non_inclusion_proof.merkle_proof.as_slice().to_vec());
        }
        for i in 0..addresses.len() {
            println!("\nunpatched {}-------------------", addresses[i]);

            let mut changelog_index = 0;
            let new_element_index = next_index + i;
            let mut low_element = IndexedElement {
                index: low_element_indices[i],
                value: BigUint::from_bytes_be(&low_element_values[i]),
                next_index: low_element_next_indices[i],
            };
            println!("unpatched low_element: {:?}", low_element);
            let mut new_element = IndexedElement {
                index: new_element_index,
                value: addresses[i].clone(),
                next_index: low_element_next_indices[i],
            };
            println!("unpatched new_element: {:?}", new_element);
            let mut low_element_proof = low_element_proofs[i].to_vec();
            let mut low_element_next_value = BigUint::from_bytes_be(&low_element_next_values[i]);

            if i > 0 {
                patch_indexed_changelogs(
                    0,
                    &mut changelog_index,
                    &mut indexed_changelog,
                    &mut low_element,
                    &mut new_element,
                    &mut low_element_next_value,
                    &mut low_element_proof,
                )
                .unwrap();
            }
            indexed_changelog.push(IndexedChangelogEntry {
                element: RawIndexedElement {
                    value: bigint_to_be_bytes_array::<32>(&low_element.value).unwrap(),
                    next_index: new_element.index,
                    next_value: bigint_to_be_bytes_array::<32>(&new_element.value).unwrap(),
                    index: low_element.index,
                },
                proof: low_element_proof.as_slice().to_vec().try_into().unwrap(),
                changelog_index: indexed_changelog.len(),
            });
            indexed_changelog.push(IndexedChangelogEntry {
                element: RawIndexedElement {
                    value: bigint_to_be_bytes_array::<32>(&new_element.value).unwrap(),
                    next_index: new_element.next_index,
                    next_value: bigint_to_be_bytes_array::<32>(&low_element_next_value).unwrap(),
                    index: new_element.index,
                },
                proof: low_element_proof.as_slice().to_vec().try_into().unwrap(),
                changelog_index: indexed_changelog.len(),
            });
            println!("patched -------------------");
            println!("changelog_index i: {}", changelog_index);
            println!("low_element: {:?}", low_element);
            println!("new_element: {:?}", new_element);
            man_indexed_array.elements[low_element.index()] = low_element.clone();
            man_indexed_array.elements[low_element.index()].next_index = new_element.index;
            man_indexed_array.elements.push(new_element);
            if i > 0 {
                let expected_low_element_value =
                    match addresses[0..i].iter().filter(|x| **x < addresses[i]).max() {
                        Some(x) => (*x).clone(),
                        None => BigUint::from_usize(0).unwrap(),
                    };
                assert_eq!(low_element.value, expected_low_element_value);
            }
        }
        println!("indexed_changelog {:?}", indexed_changelog);
        for address in addresses.iter() {
            indexed_merkle_tree.append(address).unwrap();
        }
        println!("man_indexed_array {:?}", man_indexed_array);

        assert_eq!(
            indexed_merkle_tree.indexed_array.elements,
            man_indexed_array.elements
        );
    }
}

#[test]
fn debug_test_indexed_changelog() {
    use num_traits::FromPrimitive;
    for _ in 0..1 {
        let mut indexed_merkle_tree = IndexedMerkleTree::<Poseidon, usize>::new(8, 0).unwrap();
        let mut man_indexed_array = IndexedArray::<Poseidon, usize>::new(
            BigUint::from_usize(0).unwrap(),
            BigUint::from_str(HIGHEST_ADDRESS_PLUS_ONE).unwrap(),
        );
        let mut addresses = vec![];
        for i in 0..10 {
            let address = BigUint::from_usize(101 - i).unwrap();
            addresses.push(address);
        }

        let next_index = indexed_merkle_tree.merkle_tree.rightmost_index;
        let mut indexed_changelog: Vec<IndexedChangelogEntry<usize, 8>> = Vec::new();
        let mut low_element_values = Vec::new();
        let mut low_element_indices = Vec::new();
        let mut low_element_next_indices = Vec::new();
        let mut low_element_next_values = Vec::new();
        let mut low_element_proofs: Vec<Vec<[u8; 32]>> = Vec::new();
        // get inputs
        for address in addresses.iter() {
            let non_inclusion_proof = indexed_merkle_tree
                .get_non_inclusion_proof(address)
                .unwrap();
            low_element_values.push(non_inclusion_proof.leaf_lower_range_value);
            low_element_indices.push(non_inclusion_proof.leaf_index);
            low_element_next_indices.push(non_inclusion_proof.next_index);
            low_element_next_values.push(non_inclusion_proof.leaf_higher_range_value);
            low_element_proofs.push(non_inclusion_proof.merkle_proof.as_slice().to_vec());
        }
        for i in 0..addresses.len() {
            println!("\nunpatched {}-------------------", addresses[i]);

            let mut changelog_index = 0;
            let new_element_index = next_index + i;
            let mut low_element = IndexedElement {
                index: low_element_indices[i],
                value: BigUint::from_bytes_be(&low_element_values[i]),
                next_index: low_element_next_indices[i],
            };
            println!("unpatched low_element: {:?}", low_element);
            let mut new_element = IndexedElement {
                index: new_element_index,
                value: addresses[i].clone(),
                next_index: low_element_next_indices[i],
            };
            println!("unpatched new_element: {:?}", new_element);
            let mut low_element_proof = low_element_proofs[i].to_vec();
            let mut low_element_next_value = BigUint::from_bytes_be(&low_element_next_values[i]);

            if i > 0 {
                patch_indexed_changelogs(
                    0,
                    &mut changelog_index,
                    &mut indexed_changelog,
                    &mut low_element,
                    &mut new_element,
                    &mut low_element_next_value,
                    &mut low_element_proof,
                )
                .unwrap();
            }
            indexed_changelog.push(IndexedChangelogEntry {
                element: RawIndexedElement {
                    value: bigint_to_be_bytes_array::<32>(&low_element.value).unwrap(),
                    next_index: new_element.index,
                    next_value: bigint_to_be_bytes_array::<32>(&new_element.value).unwrap(),
                    index: low_element.index,
                },
                proof: low_element_proof.as_slice().to_vec().try_into().unwrap(),
                changelog_index: indexed_changelog.len(),
            });
            indexed_changelog.push(IndexedChangelogEntry {
                element: RawIndexedElement {
                    value: bigint_to_be_bytes_array::<32>(&new_element.value).unwrap(),
                    next_index: new_element.next_index,
                    next_value: bigint_to_be_bytes_array::<32>(&low_element_next_value).unwrap(),
                    index: new_element.index,
                },
                proof: low_element_proof.as_slice().to_vec().try_into().unwrap(),
                changelog_index: indexed_changelog.len(),
            });
            man_indexed_array.elements[low_element.index()] = low_element.clone();
            man_indexed_array.elements[low_element.index()].next_index = new_element.index;
            man_indexed_array.elements.push(new_element);
            if i > 0 {
                let expected_low_element_value =
                    match addresses[0..i].iter().filter(|x| **x < addresses[i]).max() {
                        Some(x) => (*x).clone(),
                        None => BigUint::from_usize(0).unwrap(),
                    };
                assert_eq!(low_element.value, expected_low_element_value);
            }
        }
        println!("indexed_changelog {:?}", indexed_changelog);
        for address in addresses.iter() {
            indexed_merkle_tree.append(address).unwrap();
        }
        println!("man_indexed_array {:?}", man_indexed_array);
        println!("indexed_array {:?}", indexed_merkle_tree.indexed_array);

        assert_eq!(
            indexed_merkle_tree.indexed_array.elements,
            man_indexed_array.elements
        );
    }
}
