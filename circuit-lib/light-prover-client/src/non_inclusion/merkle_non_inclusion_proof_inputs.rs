use crate::{
    batch_append_with_subtrees::calculate_two_inputs_hash_chain, helpers::bigint_to_u8_32,
};
use light_indexed_merkle_tree::array::IndexedArray;
use num_bigint::{BigInt, BigUint};
use num_traits::ops::bytes::FromBytes;

#[derive(Clone, Debug)]
pub struct NonInclusionMerkleProofInputs {
    pub root: BigInt,
    pub value: BigInt,

    pub leaf_lower_range_value: BigInt,
    pub leaf_higher_range_value: BigInt,
    pub next_index: BigInt,

    pub merkle_proof_hashed_indexed_element_leaf: Vec<BigInt>,
    pub index_hashed_indexed_element_leaf: BigInt,
}

#[derive(Clone, Debug)]
pub struct NonInclusionProofInputs<'a> {
    pub public_input_hash: BigInt,
    pub inputs: &'a [NonInclusionMerkleProofInputs],
}

// TODO: eliminate use of BigInt in favor of BigUint
pub fn get_non_inclusion_proof_inputs(
    value: &[u8; 32],
    merkle_tree: &light_indexed_merkle_tree::reference::IndexedMerkleTree<
        light_hasher::Poseidon,
        usize,
    >,
    indexed_array: &IndexedArray<light_hasher::Poseidon, usize>,
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
        next_index: BigInt::from(non_inclusion_proof.next_index),
        merkle_proof_hashed_indexed_element_leaf: proof,
        index_hashed_indexed_element_leaf: BigInt::from(non_inclusion_proof.leaf_index),
    }
}

impl<'a> NonInclusionProofInputs<'a> {
    pub fn new(inputs: &'a [NonInclusionMerkleProofInputs]) -> Self {
        let public_input_hash = Self::public_input(inputs);
        Self {
            public_input_hash,
            inputs,
        }
    }

    pub fn public_input(inputs: &'a [NonInclusionMerkleProofInputs]) -> BigInt {
        let public_input_hash = calculate_two_inputs_hash_chain(
            &inputs
                .iter()
                .map(|x| bigint_to_u8_32(&x.root).unwrap())
                .collect::<Vec<_>>(),
            &inputs
                .iter()
                .map(|x| bigint_to_u8_32(&x.value).unwrap())
                .collect::<Vec<_>>(),
        );
        BigInt::from_bytes_be(num_bigint::Sign::Plus, &public_input_hash)
    }
}
