use num_bigint::BigInt;

use crate::helpers::bigint_to_u8_32;

#[derive(Clone, Debug)]
pub struct InclusionMerkleProofInputs {
    pub roots: BigInt,
    pub leaves: BigInt,
    pub in_path_indices: BigInt,
    pub in_path_elements: Vec<BigInt>,
}

impl InclusionMerkleProofInputs {
    pub fn public_inputs_arr(&self) -> [[u8; 32]; 2] {
        let roots = bigint_to_u8_32(&self.roots).unwrap();
        let leaves = bigint_to_u8_32(&self.leaves).unwrap();
        [roots, leaves]
    }
}

#[derive(Clone, Debug)]
pub struct InclusionProofInputs<'a>(pub &'a [InclusionMerkleProofInputs]);

impl InclusionProofInputs<'_> {
    pub fn public_inputs(&self) -> Vec<[u8; 32]> {
        let mut roots = Vec::new();
        let mut leaves = Vec::new();
        for input in self.0 {
            let input_arr = input.public_inputs_arr();
            roots.push(input_arr[0]);
            leaves.push(input_arr[1]);
        }
        [roots, leaves].concat()
    }
}
