use crate::{helpers::bigint_to_u8_32, proof_types::inclusion::v2::InclusionMerkleProofInputs};

impl InclusionMerkleProofInputs {
    pub fn public_inputs_arr(&self) -> [[u8; 32]; 2] {
        let root = bigint_to_u8_32(&self.root).unwrap();
        let leaf = bigint_to_u8_32(&self.leaf).unwrap();
        [root, leaf]
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
