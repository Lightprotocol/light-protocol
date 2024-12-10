use std::sync::Mutex;

use ark_std::Zero;
use light_hasher::{Hasher, Poseidon};
use light_indexed_merkle_tree::{array::IndexedArray, reference::IndexedMerkleTree};
use light_merkle_tree_reference::MerkleTree;
use log::info;
use num_bigint::{BigInt, Sign, ToBigUint};
use once_cell::{self, sync::Lazy};

use crate::{
    inclusion::{
        merkle_inclusion_proof_inputs::InclusionMerkleProofInputs, merkle_tree_info::MerkleTreeInfo,
    },
    non_inclusion::merkle_non_inclusion_proof_inputs::NonInclusionMerkleProofInputs,
};

pub static MT_PROOF_INPUTS_26: Lazy<Mutex<InclusionMerkleProofInputs>> =
    Lazy::new(|| Mutex::new(internal_inclusion_merkle_tree_inputs(26)));

pub fn inclusion_merkle_tree_inputs(mt_height: MerkleTreeInfo) -> InclusionMerkleProofInputs {
    match mt_height {
        MerkleTreeInfo::H26 => (*MT_PROOF_INPUTS_26.lock().unwrap()).clone(),
    }
}

fn internal_inclusion_merkle_tree_inputs(height: usize) -> InclusionMerkleProofInputs {
    const CANOPY: usize = 0;

    info!("initializing merkle tree");
    // SAFETY: Calling `unwrap()` when the Merkle tree parameters are corect
    // should not cause panic. Returning an error would not be compatible with
    // usafe of `once_cell::sync::Lazy` as a static variable.
    let mut merkle_tree = MerkleTree::<Poseidon>::new(height, CANOPY);
    info!("merkle tree initialized");

    info!("updating merkle tree");
    let mut bn_1: [u8; 32] = [0; 32];
    bn_1[31] = 1;
    let leaf: [u8; 32] = Poseidon::hash(&bn_1).unwrap();
    merkle_tree.append(&leaf).unwrap();
    let root1 = &merkle_tree.roots[1];
    info!("merkle tree updated");

    info!("getting proof of leaf");
    // SAFETY: Calling `unwrap()` when the Merkle tree parameters are corect
    // should not cause panic. Returning an error would not be compatible with
    // unsafe of `once_cell::sync::Lazy` as a static variable.
    let path_elements = merkle_tree
        .get_proof_of_leaf(0, true)
        .unwrap()
        .iter()
        .map(|el| BigInt::from_bytes_be(Sign::Plus, el))
        .collect::<Vec<_>>();
    info!("proof of leaf calculated");
    let leaf_bn = BigInt::from_bytes_be(Sign::Plus, &leaf);
    let root_bn = BigInt::from_bytes_be(Sign::Plus, root1);
    let path_index = BigInt::zero();

    InclusionMerkleProofInputs {
        root: root_bn,
        leaf: leaf_bn,
        path_index,
        path_elements,
    }
}

pub fn non_inclusion_merkle_tree_inputs(height: usize) -> NonInclusionMerkleProofInputs {
    const CANOPY: usize = 0;
    let mut indexed_tree = IndexedMerkleTree::<Poseidon, usize>::new(height, CANOPY).unwrap();
    let mut indexing_array = IndexedArray::<Poseidon, usize>::default();
    indexed_tree.init().unwrap();
    indexing_array.init().unwrap();

    let value = 1_u32.to_biguint().unwrap();

    let non_inclusion_proof = indexed_tree
        .get_non_inclusion_proof(&value, &indexing_array)
        .unwrap();

    NonInclusionMerkleProofInputs {
        root: BigInt::from_bytes_be(Sign::Plus, non_inclusion_proof.root.as_slice()),
        value: BigInt::from_bytes_be(Sign::Plus, &non_inclusion_proof.value),
        leaf_lower_range_value: BigInt::from_bytes_be(
            Sign::Plus,
            &non_inclusion_proof.leaf_lower_range_value,
        ),
        leaf_higher_range_value: BigInt::from_bytes_be(
            Sign::Plus,
            &non_inclusion_proof.leaf_higher_range_value,
        ),
        next_index: BigInt::from(non_inclusion_proof.next_index),
        merkle_proof_hashed_indexed_element_leaf: non_inclusion_proof
            .merkle_proof
            .iter()
            .map(|x| BigInt::from_bytes_be(Sign::Plus, x))
            .collect(),
        index_hashed_indexed_element_leaf: BigInt::from(non_inclusion_proof.leaf_index),
    }
}
