use crate::helpers::bigint_to_u8_32;
use light_hasher::{Hasher, Poseidon};
use light_merkle_tree_reference::sparse_merkle_tree::SparseMerkleTree;
use light_utils::bigint::bigint_to_be_bytes_array;
use num_bigint::{BigInt, BigUint, Sign};
use num_traits::FromPrimitive;

#[derive(Clone, Debug, Default)]
pub struct BatchAppendCircuitInputs {
    pub public_input_hash: BigInt,
    pub old_sub_tree_hash_chain: BigInt,
    pub new_sub_tree_hash_chain: BigInt,
    pub new_root: BigInt,
    pub hashchain_hash: BigInt,
    pub start_index: BigInt,
    pub tree_height: BigInt,
    pub leaves: Vec<BigInt>,
    pub subtrees: Vec<BigInt>,
}

impl BatchAppendCircuitInputs {
    pub fn public_inputs_arr(&self) -> [u8; 32] {
        bigint_to_u8_32(&self.public_input_hash).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct BatchAppendInputs<'a>(pub &'a [BatchAppendCircuitInputs]);

impl BatchAppendInputs<'_> {
    pub fn public_inputs(&self) -> Vec<[u8; 32]> {
        // Concatenate all public inputs into a single flat vector
        vec![self.0[0].public_inputs_arr()]
    }
}

pub fn get_batch_append_inputs<const HEIGHT: usize>(
    // get either from photon or mt account
    next_index: usize,
    // get from photon
    sub_trees: [[u8; 32]; HEIGHT],
    // get from queue
    leaves: Vec<[u8; 32]>,
    // get from queue
    leaves_hashchain: [u8; 32],
) -> BatchAppendCircuitInputs {
    let mut bigint_leaves = vec![];
    let old_subtrees = sub_trees;
    let old_subtree_hashchain = calculate_hash_chain(&old_subtrees);
    let mut merkle_tree = SparseMerkleTree::<Poseidon, HEIGHT>::new(sub_trees, next_index);
    let start_index =
        bigint_to_be_bytes_array::<32>(&BigUint::from_usize(next_index).unwrap()).unwrap();
    for leaf in leaves.iter() {
        merkle_tree.append(*leaf);
        bigint_leaves.push(BigInt::from_bytes_be(Sign::Plus, leaf));
    }

    let new_root = BigInt::from_signed_bytes_be(merkle_tree.root().as_slice());

    let new_subtree_hashchain = calculate_hash_chain(&merkle_tree.get_subtrees());

    let public_input_hash = calculate_hash_chain(&[
        old_subtree_hashchain,
        new_subtree_hashchain,
        merkle_tree.root(),
        leaves_hashchain,
        start_index,
    ]);

    BatchAppendCircuitInputs {
        subtrees: old_subtrees
            .iter()
            .map(|subtree| BigInt::from_bytes_be(Sign::Plus, subtree))
            .collect(),
        old_sub_tree_hash_chain: BigInt::from_bytes_be(Sign::Plus, &old_subtree_hashchain),
        new_sub_tree_hash_chain: BigInt::from_bytes_be(Sign::Plus, &new_subtree_hashchain),
        leaves: bigint_leaves,
        new_root,
        public_input_hash: BigInt::from_bytes_be(Sign::Plus, &public_input_hash),
        start_index: BigInt::from_bytes_be(Sign::Plus, &start_index),
        hashchain_hash: BigInt::from_bytes_be(Sign::Plus, &leaves_hashchain),
        tree_height: BigInt::from_usize(merkle_tree.get_height()).unwrap(),
    }
}

pub fn calculate_hash_chain(hashes: &[[u8; 32]]) -> [u8; 32] {
    if hashes.is_empty() {
        return [0u8; 32];
    }

    if hashes.len() == 1 {
        return hashes[0];
    }

    let mut hash_chain = hashes[0];
    for hash in hashes.iter().skip(1) {
        hash_chain = Poseidon::hashv(&[&hash_chain, hash]).unwrap();
    }
    hash_chain
}
