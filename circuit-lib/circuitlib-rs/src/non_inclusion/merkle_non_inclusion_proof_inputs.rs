use crate::helpers::bigint_to_u8_32;
use light_indexed_merkle_tree::array::IndexedArray;
use num_bigint::{BigInt, BigUint};
use num_traits::ops::bytes::FromBytes;

#[derive(Clone, Debug)]
pub struct NonInclusionMerkleProofInputs {
    pub root: BigInt,
    pub value: BigInt,

    pub leaf_lower_range_value: BigInt,
    pub leaf_higher_range_value: BigInt,
    pub leaf_index: BigInt,

    pub merkle_proof_hashed_indexed_element_leaf: Vec<BigInt>,
    pub index_hashed_indexed_element_leaf: BigInt,
}

impl NonInclusionMerkleProofInputs {
    pub fn public_inputs_arr(&self) -> [[u8; 32]; 2] {
        let root = bigint_to_u8_32(&self.root).unwrap();
        let value = bigint_to_u8_32(&self.value).unwrap();
        [root, value]
    }
}

#[derive(Clone, Debug)]
pub struct NonInclusionProofInputs<'a>(pub &'a [NonInclusionMerkleProofInputs]);

// TODO: eliminate use of BigInt in favor of BigUint
pub fn get_non_inclusion_proof_inputs<const INDEXED_ARRAY_SIZE: usize>(
    value: &[u8; 32],
    merkle_tree: &light_indexed_merkle_tree::reference::IndexedMerkleTree<
        light_hasher::Poseidon,
        usize,
    >,
    indexed_array: &IndexedArray<light_hasher::Poseidon, usize, INDEXED_ARRAY_SIZE>,
) -> NonInclusionMerkleProofInputs {
    let non_inclusion_proof = merkle_tree
        .get_non_inclusion_proof(&BigUint::from_be_bytes(value), indexed_array)
        .unwrap();
    let proof = non_inclusion_proof
        .merkle_proof
        .iter()
        .map(|x| BigInt::from_be_bytes(x))
        .collect();
    NonInclusionMerkleProofInputs {
        root: BigInt::from_be_bytes(merkle_tree.root().as_slice()),
        value: BigInt::from_be_bytes(value),
        leaf_lower_range_value: BigInt::from_be_bytes(&non_inclusion_proof.leaf_lower_range_value),
        leaf_higher_range_value: BigInt::from_be_bytes(
            &non_inclusion_proof.leaf_higher_range_value,
        ),
        leaf_index: BigInt::from(non_inclusion_proof.next_index),
        merkle_proof_hashed_indexed_element_leaf: proof,
        index_hashed_indexed_element_leaf: BigInt::from(non_inclusion_proof.leaf_index),
    }
}
