use std::{collections::HashMap, convert::TryInto};

use ark_circom::circom::Inputs;
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

impl<'a> TryInto<HashMap<String, Inputs>> for InclusionProofInputs<'a> {
    type Error = std::io::Error;

    fn try_into(self) -> Result<HashMap<String, Inputs>, Self::Error> {
        let mut inputs: HashMap<String, Inputs> = HashMap::new();
        let mut roots: Vec<BigInt> = Vec::new();
        let mut leaves: Vec<BigInt> = Vec::new();
        let mut indices: Vec<BigInt> = Vec::new();
        let mut els: Vec<Vec<BigInt>> = Vec::new();

        for input in self.0 {
            roots.push(input.roots.clone());
            leaves.push(input.leaves.clone());
            indices.push(input.in_path_indices.clone());
            els.push(input.in_path_elements.clone());
        }

        inputs
            .entry("roots".to_string())
            .or_insert_with(|| Inputs::BigIntVec(roots));
        inputs
            .entry("leaves".to_string())
            .or_insert_with(|| Inputs::BigIntVec(leaves));
        inputs
            .entry("inPathIndices".to_string())
            .or_insert_with(|| Inputs::BigIntVec(indices));
        inputs
            .entry("inPathElements".to_string())
            .or_insert_with(|| Inputs::BigIntVecVec(els));

        Ok(inputs)
    }
}

#[cfg(test)]
mod tests {
    use ark_std::Zero;

    use super::*;

    #[test]
    fn test_conversion_to_hashmap() {
        let zero_input = InclusionMerkleProofInputs {
            leaves: BigInt::zero(),
            roots: BigInt::zero(),
            in_path_elements: vec![BigInt::zero()],
            in_path_indices: BigInt::zero(),
        };

        let inputs: [InclusionMerkleProofInputs; 2] = [zero_input.clone(), zero_input.clone()];
        let proof_inputs = InclusionProofInputs(&inputs);
        let result: HashMap<String, Inputs> = proof_inputs.try_into().unwrap();
        assert_eq!(result.len(), inputs.len() * 2);
    }
}
