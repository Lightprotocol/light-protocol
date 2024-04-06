use std::sync::Mutex;

use ark_ff::{BigInteger, BigInteger256};
use ark_std::Zero;
use light_hasher::{Hasher, Poseidon};
use light_indexed_merkle_tree::{array::IndexingArray, reference::IndexedMerkleTree};
use light_merkle_tree_reference::MerkleTree;
use log::info;
use num_bigint::{BigInt, Sign};
use once_cell::{self, sync::Lazy};

use crate::{
    inclusion::{
        merkle_inclusion_proof_inputs::InclusionMerkleProofInputs, merkle_tree_info::MerkleTreeInfo,
    },
    non_inclusion::merkle_non_inclusion_proof_inputs::NonInclusionMerkleProofInputs,
};

pub static MT_PROOF_INPUTS_26: Lazy<Mutex<InclusionMerkleProofInputs>> =
    Lazy::new(|| Mutex::new(inclusion_merkle_tree_inputs_26()));

pub fn inclusion_merkle_tree_inputs(mt_height: MerkleTreeInfo) -> InclusionMerkleProofInputs {
    match mt_height {
        MerkleTreeInfo::H26 => (*MT_PROOF_INPUTS_26.lock().unwrap()).clone(),
    }
}

fn inclusion_merkle_tree_inputs_26() -> InclusionMerkleProofInputs {
    const HEIGHT: usize = 26;
    const ROOTS: usize = 1;
    const CANOPY: usize = 0;

    info!("initializing merkle tree");
    // SAFETY: Calling `unwrap()` when the Merkle tree parameters are corect
    // should not cause panic. Returning an error would not be compatible with
    // usafe of `once_cell::sync::Lazy` as a static variable.
    let mut merkle_tree = MerkleTree::<Poseidon>::new(HEIGHT, ROOTS, CANOPY).unwrap();
    info!("merkle tree initialized");

    info!("updating merkle tree");
    let mut bn_1: [u8; 32] = [0; 32];
    bn_1[31] = 1;
    let leaf: [u8; 32] = Poseidon::hash(&bn_1).unwrap();
    merkle_tree.update(&leaf, 0).unwrap();
    let root1 = &merkle_tree.roots[1];
    info!("merkle tree updated");

    info!("getting proof of leaf");
    // SAFETY: Calling `unwrap()` when the Merkle tree parameters are corect
    // should not cause panic. Returning an error would not be compatible with
    // usafe of `once_cell::sync::Lazy` as a static variable.
    let in_path_elements = merkle_tree
        .get_proof_of_leaf(0, true)
        .unwrap()
        .iter()
        .map(|el| BigInt::from_bytes_be(Sign::Plus, el))
        .collect::<Vec<_>>();
    info!("proof of leaf calculated");
    let leaf_bn = BigInt::from_bytes_be(Sign::Plus, &leaf);
    let root_bn = BigInt::from_bytes_be(Sign::Plus, root1);
    let in_path_indices = BigInt::zero();

    InclusionMerkleProofInputs {
        roots: root_bn,
        leaves: leaf_bn,
        in_path_indices,
        in_path_elements,
    }
}

pub fn non_inclusion_merkle_tree_inputs_26() -> NonInclusionMerkleProofInputs {
    const HEIGHT: usize = 26;
    const ROOTS: usize = 1;
    const CANOPY: usize = 0;
    let mut indexed_tree =
        IndexedMerkleTree::<Poseidon, usize, BigInteger256>::new(HEIGHT, ROOTS, CANOPY).unwrap();
    let mut indexing_array = IndexingArray::<Poseidon, usize, BigInteger256, 1024>::default();

    let bundle1 = indexing_array.append(BigInteger256::from(1_u32)).unwrap();
    indexed_tree
        .update(
            &bundle1.new_low_element,
            &bundle1.new_element,
            &bundle1.new_element_next_value,
        )
        .unwrap();

    let bundle3 = indexing_array.append(BigInteger256::from(3_u32)).unwrap();
    indexed_tree
        .update(
            &bundle3.new_low_element,
            &bundle3.new_element,
            &bundle3.new_element_next_value,
        )
        .unwrap();

    let new_low_element = bundle3.new_low_element;
    let new_element = bundle3.new_element;
    let _new_element_next_value = bundle3.new_element_next_value;
    let root = indexed_tree.merkle_tree.roots.last().unwrap();
    let mut non_included_value = [0u8; 32];
    non_included_value[31] = 2;

    let leaf_lower_range_value = new_low_element.value.to_bytes_be();
    let leaf_index = new_element.index;
    let leaf_higher_range_value = new_element.value.to_bytes_be();
    let merkle_proof_hashed_indexed_element_leaf = indexed_tree
        .get_proof_of_leaf(new_low_element.index, true)
        .ok()
        .map(|bounded_vec| {
            bounded_vec
                .iter()
                .map(|item| BigInt::from_bytes_be(Sign::Plus, item))
                .collect()
        })
        .unwrap();
    let index_hashed_indexed_element_leaf = new_low_element.index;

    NonInclusionMerkleProofInputs {
        root: BigInt::from_bytes_be(Sign::Plus, root),
        value: BigInt::from_bytes_be(Sign::Plus, &non_included_value),
        leaf_lower_range_value: BigInt::from_bytes_be(Sign::Plus, &leaf_lower_range_value),
        leaf_higher_range_value: BigInt::from_bytes_be(Sign::Plus, &leaf_higher_range_value),
        leaf_index: BigInt::from(leaf_index),
        merkle_proof_hashed_indexed_element_leaf,
        index_hashed_indexed_element_leaf: BigInt::from(index_hashed_indexed_element_leaf),
    }
}
