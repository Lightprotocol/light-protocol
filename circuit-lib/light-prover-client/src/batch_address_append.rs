use num_bigint::BigUint;
use light_indexed_merkle_tree::{array::IndexedArray, reference::IndexedMerkleTree};
use light_hasher::Poseidon;
use light_utils::bigint::bigint_to_be_bytes_array;
use crate::helpers::hash_chain;

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
pub fn get_batch_address_append_inputs_from_tree(
    current_root: [u8; 32],
    addresses: Vec<BigUint>,
    start_index: usize,
    tree_height: usize,
    
    low_element_values: Vec<[u8; 32]>,
    low_element_next_values: Vec<[u8; 32]>,
    low_element_indices: Vec<usize>,
    low_element_next_indices: Vec<usize>,

    low_element_proofs: Vec<Vec<[u8; 32]>>,
    new_element_proofs: Vec<Vec<[u8; 32]>>,

    new_root: [u8; 32],
) -> BatchAddressAppendInputs {
    let addresses_bytes = addresses
        .iter()
        .map(|x| bigint_to_be_bytes_array::<32>(x).unwrap())
        .collect::<Vec<_>>();
    
    let leaves_hashchain = hash_chain(&addresses_bytes);
    let hash_chain_inputs = vec![
        current_root,
        new_root,
        leaves_hashchain,
        bigint_to_be_bytes_array::<32>(&start_index.into()).unwrap(),
    ];
    let public_input_hash = hash_chain(hash_chain_inputs.as_slice());

    BatchAddressAppendInputs {
        batch_size: addresses.len(),
        hashchain_hash: BigUint::from_bytes_be(&leaves_hashchain),
        low_element_values: low_element_values
            .iter()
            .map(|v| BigUint::from_bytes_be(v))
            .collect(),
        low_element_indices: low_element_indices
            .iter()
            .map(|&i| BigUint::from(i))
            .collect(),
        low_element_next_indices: low_element_next_indices
            .iter()
            .map(|&i| BigUint::from(i))
            .collect(),
        low_element_next_values: low_element_next_values
            .iter()
            .map(|v| BigUint::from_bytes_be(v))
            .collect(),
        low_element_proofs: low_element_proofs
            .iter()
            .map(|proof| proof.iter().map(|p| BigUint::from_bytes_be(p)).collect())
            .collect(),
        new_element_values: addresses,
        new_element_proofs: new_element_proofs
            .iter()
            .map(|proof| proof.iter().map(|p| BigUint::from_bytes_be(p)).collect())
            .collect(),
        new_root: BigUint::from_bytes_be(&new_root),
        old_root: BigUint::from_bytes_be(&current_root),
        public_input_hash: BigUint::from_bytes_be(&public_input_hash),
        start_index,
        tree_height,
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
    let mut relayer_merkle_tree = IndexedMerkleTree::<Poseidon, usize>::new(tree_height, 0).unwrap();
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

        low_element_values.push(BigUint::from_bytes_be(&non_inclusion_proof.leaf_lower_range_value));
        low_element_indices.push(non_inclusion_proof.leaf_index.into());
        low_element_next_indices.push(non_inclusion_proof.next_index.into());
        low_element_next_values.push(BigUint::from_bytes_be(&non_inclusion_proof.leaf_higher_range_value));
        
        let proof: Vec<BigUint> = non_inclusion_proof.merkle_proof
            .iter()
            .map(|proof| BigUint::from_bytes_be(proof))
            .collect();
        low_element_proofs.push(proof);

        relayer_merkle_tree
            .append(address, &mut relayer_indexing_array)
            .unwrap();

        let new_proof = relayer_merkle_tree
            .get_proof_of_leaf(relayer_merkle_tree.merkle_tree.rightmost_index-1, true)
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