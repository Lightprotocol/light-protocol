use std::sync::Mutex;

use ark_std::Zero;
use light_hasher::{Hasher, Poseidon};
use light_merkle_tree_reference::MerkleTree;
use log::info;
use num_bigint::{BigInt, Sign};
use once_cell::{self, sync::Lazy};

use crate::merkle_proof_inputs::{MerkleTreeInfo, MerkleTreeProofInput};

pub static MT_PROOF_INPUTS_26: Lazy<Mutex<MerkleTreeProofInput>> =
    Lazy::new(|| Mutex::new(merkle_tree_inputs_26()));

pub fn merkle_tree_inputs(mt_height: MerkleTreeInfo) -> MerkleTreeProofInput {
    match mt_height {
        MerkleTreeInfo::H26 => (*MT_PROOF_INPUTS_26.lock().unwrap()).clone(),
    }
}

fn merkle_tree_inputs_26() -> MerkleTreeProofInput {
    const HEIGHT: usize = 26;
    const ROOTS: usize = 1;

    info!("initializing merkle tree");
    let mut merkle_tree = MerkleTree::<Poseidon, HEIGHT, ROOTS>::new().unwrap();
    info!("merkle tree initialized");

    info!("updating merkle tree");
    let mut bn_1: [u8; 32] = [0; 32];
    bn_1[31] = 1;
    let leaf: [u8; 32] = Poseidon::hash(&bn_1).unwrap();
    merkle_tree.update(&leaf, 0).unwrap();
    let root1 = &merkle_tree.roots[1];
    info!("merkle tree updated");

    info!("getting proof of leaf");
    let proof_of_leaf = merkle_tree
        .get_proof_of_leaf(0)
        .map(|el| BigInt::from_bytes_be(Sign::Plus, &el));
    info!("proof of leaf calculated");
    let leaf_bn = BigInt::from_bytes_be(Sign::Plus, &leaf);
    let root_bn = BigInt::from_bytes_be(Sign::Plus, root1);
    let in_path_indices = BigInt::zero();
    let in_path_elements = proof_of_leaf.to_vec();

    MerkleTreeProofInput {
        leaf: leaf_bn,
        root: root_bn,
        in_path_indices,
        in_path_elements,
    }
}
