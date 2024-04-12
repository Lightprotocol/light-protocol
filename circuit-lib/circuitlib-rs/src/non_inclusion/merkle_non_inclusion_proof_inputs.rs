use std::{collections::HashMap, convert::TryInto};

use crate::helpers::bigint_to_u8_32;
use ark_circom::circom::Inputs;
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

impl NonInclusionProofInputs<'_> {
    pub fn public_inputs(&self) -> Vec<[u8; 32]> {
        let mut roots = Vec::new();
        let mut values = Vec::new();
        for input in self.0 {
            let input_arr = input.public_inputs_arr();
            roots.push(input_arr[0]);
            values.push(input_arr[1]);
        }
        [roots, values].concat()
    }
}

impl<'a> TryInto<HashMap<String, Inputs>> for NonInclusionProofInputs<'a> {
    type Error = std::io::Error;

    fn try_into(self) -> Result<HashMap<String, Inputs>, Self::Error> {
        let mut inputs: HashMap<String, Inputs> = HashMap::new();

        let mut roots: Vec<BigInt> = Vec::new();
        let mut values: Vec<BigInt> = Vec::new();

        let mut leaf_lower_range_values: Vec<BigInt> = Vec::new();
        let mut leaf_higher_range_values: Vec<BigInt> = Vec::new();
        let mut leaf_indices: Vec<BigInt> = Vec::new();

        let mut index_hashed_indexed_element_leafs: Vec<BigInt> = Vec::new();
        let mut merkle_proof_hashed_indexed_element_leafs: Vec<Vec<BigInt>> = Vec::new();

        for input in self.0 {
            roots.push(input.root.clone());
            values.push(input.value.clone());

            leaf_lower_range_values.push(input.leaf_lower_range_value.clone());
            leaf_higher_range_values.push(input.leaf_higher_range_value.clone());
            leaf_indices.push(input.leaf_index.clone());

            index_hashed_indexed_element_leafs
                .push(input.index_hashed_indexed_element_leaf.clone());
            merkle_proof_hashed_indexed_element_leafs
                .push(input.merkle_proof_hashed_indexed_element_leaf.clone());
        }

        inputs
            .entry("root".to_string())
            .or_insert_with(|| Inputs::BigIntVec(roots));
        inputs
            .entry("value".to_string())
            .or_insert_with(|| Inputs::BigIntVec(values));

        inputs
            .entry("leafLowerRangeValue".to_string())
            .or_insert_with(|| Inputs::BigIntVec(leaf_lower_range_values));
        inputs
            .entry("leafHigherRangeValue".to_string())
            .or_insert_with(|| Inputs::BigIntVec(leaf_higher_range_values));
        inputs
            .entry("leafIndex".to_string())
            .or_insert_with(|| Inputs::BigIntVec(leaf_indices));

        inputs
            .entry("merkleProofHashedIndexedElementLeaf".to_string())
            .or_insert_with(|| Inputs::BigIntVecVec(merkle_proof_hashed_indexed_element_leafs));
        inputs
            .entry("indexHashedIndexedElementLeaf".to_string())
            .or_insert_with(|| Inputs::BigIntVec(index_hashed_indexed_element_leafs));

        Ok(inputs)
    }
}

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

#[cfg(test)]
mod tests {
    use ark_std::Zero;

    use super::*;

    #[test]
    fn test_conversion_to_hashmap() {
        let zero_input = NonInclusionMerkleProofInputs {
            root: BigInt::zero(),
            value: BigInt::zero(),
            leaf_lower_range_value: BigInt::zero(),
            leaf_higher_range_value: BigInt::zero(),
            leaf_index: BigInt::zero(),
            merkle_proof_hashed_indexed_element_leaf: vec![BigInt::zero()],
            index_hashed_indexed_element_leaf: BigInt::zero(),
        };

        let inputs: [NonInclusionMerkleProofInputs; 2] = [zero_input.clone(), zero_input.clone()];
        let proof_inputs = NonInclusionProofInputs(&inputs);
        let result: HashMap<String, Inputs> = proof_inputs.try_into().unwrap();
        let roots = result.get("root").unwrap();
        match roots {
            Inputs::BigIntVec(n) => {
                assert_eq!(n.len(), 2);
                assert_eq!(n[0], zero_input.root);
                assert_eq!(n[1], zero_input.root);
            }
            _ => panic!("Expected BigIntVec"),
        }
    }
}
