use crate::helpers::{compute_root_from_merkle_proof, hash_chain};
use light_bounded_vec::BoundedVec;
use light_concurrent_merkle_tree::changelog::ChangelogEntry;
use light_concurrent_merkle_tree::event::RawIndexedElement;
use light_hasher::Poseidon;
use light_indexed_merkle_tree::array::IndexedElement;
use light_indexed_merkle_tree::changelog::IndexedChangelogEntry;
use light_indexed_merkle_tree::errors::IndexedMerkleTreeError;
use light_indexed_merkle_tree::{array::IndexedArray, reference::IndexedMerkleTree};
use light_merkle_tree_reference::sparse_merkle_tree::SparseMerkleTree;
use light_utils::bigint::bigint_to_be_bytes_array;
use num_bigint::BigUint;

#[derive(Debug, Clone)]
pub struct BatchAddressAppendInputs {
    pub batch_size: usize,
    pub hashchain_hash: BigUint,
    pub low_element_values: Vec<BigUint>,
    pub low_element_indices: Vec<BigUint>,
    pub low_element_next_indices: Vec<BigUint>,
    pub low_element_next_values: Vec<BigUint>,
    pub low_element_proofs: Vec<Vec<BigUint>>,
    pub new_element_values: Vec<BigUint>,
    pub new_element_proofs: Vec<Vec<BigUint>>,
    pub new_root: BigUint,
    pub old_root: BigUint,
    pub public_input_hash: BigUint,
    pub start_index: usize,
    pub tree_height: usize,
}

#[allow(clippy::too_many_arguments)]
pub fn get_batch_address_append_inputs_from_tree<const HEIGHT: usize>(
    next_index: usize,
    current_root: [u8; 32],
    low_element_values: Vec<[u8; 32]>,
    low_element_next_values: Vec<[u8; 32]>,
    low_element_indices: Vec<usize>,
    low_element_next_indices: Vec<usize>,
    low_element_proofs: Vec<Vec<[u8; 32]>>,
    new_element_values: Vec<[u8; 32]>,
    subtrees: [[u8; 32]; HEIGHT],
) -> BatchAddressAppendInputs {
    let mut new_root = [0u8; 32];
    let mut low_element_circuit_merkle_proofs = vec![];
    let mut new_element_circuit_merkle_proofs = vec![];
    let mut changelog: Vec<ChangelogEntry<HEIGHT>> = Vec::new();

    let mut indexed_changelog: Vec<IndexedChangelogEntry<u16, HEIGHT>> = Vec::new();

    let mut patched_low_element_next_values: Vec<[u8; 32]> = Vec::new();
    let mut patched_low_element_next_indices: Vec<usize> = Vec::new();

    let mut merkle_tree = SparseMerkleTree::<Poseidon, HEIGHT>::new(subtrees, next_index);

    for i in 0..low_element_values.len() {
        let mut changelog_index = 0;

        let new_element_index = next_index + i;
        let mut low_element: IndexedElement<u16> = IndexedElement {
            index: low_element_indices[i] as u16,
            value: BigUint::from_bytes_be(&low_element_values[i]),
            next_index: low_element_next_indices[i] as u16,
        };

        let mut new_element: IndexedElement<u16> = IndexedElement {
            index: new_element_index as u16,
            value: BigUint::from_bytes_be(&new_element_values[i]),
            next_index: low_element_next_indices[i] as u16,
        };

        let mut low_element_proof: BoundedVec<[u8; 32]> =
            BoundedVec::from_slice(low_element_proofs[i].as_slice());
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
        patched_low_element_next_values
            .push(bigint_to_be_bytes_array::<32>(&low_element_next_value).unwrap());
        patched_low_element_next_indices.push(low_element.next_index as usize);

        let new_low_element: IndexedElement<u16> = IndexedElement {
            index: low_element.index,
            value: low_element.value,
            next_index: new_element_index as u16,
        };
        let new_low_element_raw = RawIndexedElement {
            value: bigint_to_be_bytes_array::<32>(&new_low_element.value).unwrap(),
            next_index: new_low_element.next_index,
            next_value: bigint_to_be_bytes_array::<32>(&new_element.value).unwrap(),
            index: new_low_element.index,
        };

        let low_element_changelog_entry = IndexedChangelogEntry {
            element: new_low_element_raw,
            proof: low_element_proof.as_slice()[..HEIGHT].try_into().unwrap(),
            changelog_index,
        };

        indexed_changelog.push(low_element_changelog_entry);

        {
            if i > 0 {
                for change_log_entry in changelog.iter().skip(changelog_index + 1) {
                    change_log_entry
                        .update_proof(low_element_indices[i], &mut low_element_proof)
                        .unwrap();
                }
            }
            let merkle_proof = low_element_proof.to_array().unwrap();
            let new_low_leaf_hash = new_low_element
                .hash::<Poseidon>(&new_element.value)
                .unwrap();
            let (_, changelog_entry) = compute_root_from_merkle_proof(
                new_low_leaf_hash,
                &merkle_proof,
                new_low_element.index as u32,
            );
            changelog.push(changelog_entry);
            low_element_circuit_merkle_proofs.push(
                merkle_proof
                    .iter()
                    .map(|hash| BigUint::from_bytes_be(hash))
                    .collect(),
            );
        }

        {
            let new_element_value = low_element_next_value;
            let new_element_leaf_hash = new_element.hash::<Poseidon>(&new_element_value).unwrap();
            let proof = merkle_tree.append(new_element_leaf_hash);

            let mut bounded_vec_merkle_proof = BoundedVec::from_slice(proof.as_slice());
            let current_index = next_index + i;

            for change_log_entry in changelog.iter() {
                change_log_entry
                    .update_proof(current_index, &mut bounded_vec_merkle_proof)
                    .unwrap();
            }

            let reference_root = compute_root_from_merkle_proof(
                new_element_leaf_hash,
                &proof,
                (next_index + i) as u32,
            );
            assert_eq!(merkle_tree.root(), reference_root.0);

            let merkle_proof_array = bounded_vec_merkle_proof.to_array().unwrap();

            let (updated_root, changelog_entry) = compute_root_from_merkle_proof(
                new_element_leaf_hash,
                &merkle_proof_array,
                (next_index + i) as u32,
            );
            new_root = updated_root;

            changelog.push(changelog_entry);
            new_element_circuit_merkle_proofs.push(
                merkle_proof_array
                    .iter()
                    .map(|hash| BigUint::from_bytes_be(hash))
                    .collect(),
            );

            let new_element_raw = RawIndexedElement {
                value: bigint_to_be_bytes_array::<32>(&new_element.value).unwrap(),
                next_index: new_element.next_index,
                next_value: bigint_to_be_bytes_array::<32>(&new_low_element.value).unwrap(),
                index: new_element.index,
            };

            let new_element_changelog_entry = IndexedChangelogEntry {
                element: new_element_raw,
                proof: proof.as_slice()[..HEIGHT].try_into().unwrap(),
                changelog_index: 0,
            };
            indexed_changelog.push(new_element_changelog_entry);
        }
    }

    let leaves_hashchain = hash_chain(&new_element_values);
    let hash_chain_inputs = vec![
        current_root,
        new_root,
        leaves_hashchain,
        bigint_to_be_bytes_array::<32>(&next_index.into()).unwrap(),
    ];
    let public_input_hash = hash_chain(hash_chain_inputs.as_slice());

    BatchAddressAppendInputs {
        batch_size: new_element_values.len(),
        hashchain_hash: BigUint::from_bytes_be(&leaves_hashchain),
        low_element_values: low_element_values
            .iter()
            .map(|v| BigUint::from_bytes_be(v))
            .collect(),
        low_element_indices: low_element_indices
            .iter()
            .map(|&i| BigUint::from(i))
            .collect(),
        low_element_next_indices: patched_low_element_next_indices
            .iter()
            .map(|&i| BigUint::from(i))
            .collect(),
        low_element_next_values: patched_low_element_next_values
            .iter()
            .map(|v| BigUint::from_bytes_be(v))
            .collect(),
        low_element_proofs: low_element_circuit_merkle_proofs,
        new_element_values: new_element_values
            .iter()
            .map(|v| BigUint::from_bytes_be(v))
            .collect(),
        new_element_proofs: new_element_circuit_merkle_proofs,
        new_root: BigUint::from_bytes_be(&new_root),
        old_root: BigUint::from_bytes_be(&current_root),
        public_input_hash: BigUint::from_bytes_be(&public_input_hash),
        start_index: next_index,
        tree_height: HEIGHT,
    }
}

// Keep this for testing purposes
pub fn get_test_batch_address_append_inputs(
    addresses: Vec<BigUint>,
    start_index: usize,
    tree_height: usize,
) -> BatchAddressAppendInputs {
    let mut relayer_indexing_array = IndexedArray::<Poseidon, usize>::default();
    relayer_indexing_array.init().unwrap();
    let mut relayer_merkle_tree =
        IndexedMerkleTree::<Poseidon, usize>::new(tree_height, 0).unwrap();
    relayer_merkle_tree.init().unwrap();

    let old_root = relayer_merkle_tree.root();

    let mut low_element_values = Vec::new();
    let mut low_element_indices = Vec::new();
    let mut low_element_next_indices = Vec::new();
    let mut low_element_next_values = Vec::new();
    let mut low_element_proofs = Vec::new();
    let mut new_element_values = Vec::new();
    let mut new_element_proofs = Vec::new();

    for address in &addresses {
        let non_inclusion_proof = relayer_merkle_tree
            .get_non_inclusion_proof(address, &relayer_indexing_array)
            .unwrap();
        relayer_merkle_tree
            .verify_non_inclusion_proof(&non_inclusion_proof)
            .unwrap();

        low_element_values.push(BigUint::from_bytes_be(
            &non_inclusion_proof.leaf_lower_range_value,
        ));
        low_element_indices.push(non_inclusion_proof.leaf_index.into());
        low_element_next_indices.push(non_inclusion_proof.next_index.into());
        low_element_next_values.push(BigUint::from_bytes_be(
            &non_inclusion_proof.leaf_higher_range_value,
        ));

        let proof: Vec<BigUint> = non_inclusion_proof
            .merkle_proof
            .iter()
            .map(|proof| BigUint::from_bytes_be(proof))
            .collect();
        low_element_proofs.push(proof);

        relayer_merkle_tree
            .append(address, &mut relayer_indexing_array)
            .unwrap();

        let new_proof = relayer_merkle_tree
            .get_proof_of_leaf(relayer_merkle_tree.merkle_tree.rightmost_index - 1, true)
            .unwrap();

        let new_proof: Vec<BigUint> = new_proof
            .iter()
            .map(|proof| BigUint::from_bytes_be(proof))
            .collect();
        new_element_proofs.push(new_proof);
        new_element_values.push(address.clone());
    }

    let new_root = relayer_merkle_tree.root();

    // Create hashchain
    let addresses_bytes = addresses
        .iter()
        .map(|x| bigint_to_be_bytes_array::<32>(x).unwrap())
        .collect::<Vec<_>>();

    let leaves_hashchain = hash_chain(&addresses_bytes);
    let hash_chain_inputs = vec![
        old_root,
        new_root,
        leaves_hashchain,
        bigint_to_be_bytes_array::<32>(&start_index.into()).unwrap(),
    ];
    let public_input_hash = hash_chain(hash_chain_inputs.as_slice());

    BatchAddressAppendInputs {
        batch_size: addresses.len(),
        hashchain_hash: BigUint::from_bytes_be(&leaves_hashchain),
        low_element_values,
        low_element_indices,
        low_element_next_indices,
        low_element_next_values,
        low_element_proofs,
        new_element_values,
        new_element_proofs,
        new_root: BigUint::from_bytes_be(&new_root),
        old_root: BigUint::from_bytes_be(&old_root),
        public_input_hash: BigUint::from_bytes_be(&public_input_hash),
        start_index,
        tree_height,
    }
}

pub fn patch_indexed_changelogs<const HEIGHT: usize>(
    indexed_changelog_index: usize,
    changelog_index: &mut usize,
    indexed_changelogs: &mut Vec<IndexedChangelogEntry<u16, HEIGHT>>,
    low_element: &mut IndexedElement<u16>,
    new_element: &mut IndexedElement<u16>,
    low_element_next_value: &mut BigUint,
    low_leaf_proof: &mut BoundedVec<[u8; 32]>,
) -> Result<(), IndexedMerkleTreeError> {
    let next_indexed_changelog_indices: Vec<usize> = (*indexed_changelogs)
        .iter()
        .enumerate()
        .filter_map(|(index, changelog_entry)| {
            if changelog_entry.element.index == low_element.index {
                Some((indexed_changelog_index + index) % indexed_changelogs.len())
            } else {
                None
            }
        })
        .collect();

    let mut new_low_element = None;

    for next_indexed_changelog_index in next_indexed_changelog_indices {
        let changelog_entry = &mut indexed_changelogs[next_indexed_changelog_index];

        let next_element_value = BigUint::from_bytes_be(&changelog_entry.element.next_value);
        if next_element_value < new_element.value {
            // If the next element is lower than the current element, it means
            // that it should become the low element.
            //
            // Save it and break the loop.
            new_low_element = Some((
                (next_indexed_changelog_index + 1) % indexed_changelogs.len(),
                next_element_value,
            ));
            break;
        }

        // Patch the changelog index.
        *changelog_index = changelog_entry.changelog_index;

        // Patch the `next_index` of `new_element`.
        new_element.next_index = changelog_entry.element.next_index;
        // Patch the element.
        low_element.update_from_raw_element(&changelog_entry.element);
        // Patch the next value.
        *low_element_next_value = BigUint::from_bytes_be(&changelog_entry.element.next_value);
        // Patch the proof.
        for i in 0..low_leaf_proof.len() {
            low_leaf_proof[i] = changelog_entry.proof[i];
        }
    }

    // If we found a new low element.
    if let Some((new_low_element_changelog_index, new_low_element)) = new_low_element {
        let new_low_element_changelog_entry = &indexed_changelogs[new_low_element_changelog_index];
        *changelog_index = new_low_element_changelog_entry.changelog_index;
        *low_element = IndexedElement {
            index: new_low_element_changelog_entry.element.index,
            value: new_low_element.clone(),
            next_index: new_low_element_changelog_entry.element.next_index,
        };

        for i in 0..low_leaf_proof.len() {
            low_leaf_proof[i] = new_low_element_changelog_entry.proof[i];
        }
        new_element.next_index = low_element.next_index;

        // Start the patching process from scratch for the new low element.
        patch_indexed_changelogs(
            new_low_element_changelog_index,
            changelog_index,
            indexed_changelogs,
            new_element,
            low_element,
            low_element_next_value,
            low_leaf_proof,
        )?
    }

    Ok(())
}
