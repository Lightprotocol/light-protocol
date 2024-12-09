use crate::{batch_append_with_subtrees::calculate_hash_chain, helpers::bigint_to_u8_32};
use num_bigint::BigInt;

#[derive(Clone, Debug)]
pub struct InclusionMerkleProofInputs {
    pub root: BigInt,
    pub leaf: BigInt,
    pub path_index: BigInt,
    pub path_elements: Vec<BigInt>,
}

impl InclusionMerkleProofInputs {
    pub fn public_inputs_arr(&self) -> [[u8; 32]; 2] {
        let root = bigint_to_u8_32(&self.root).unwrap();
        let leaf = bigint_to_u8_32(&self.leaf).unwrap();
        [root, leaf]
    }
}

#[derive(Clone, Debug)]
pub struct InclusionProofInputs<'a> {
    pub public_input_hash: BigInt,
    pub inputs: &'a [InclusionMerkleProofInputs],
}

impl<'a> InclusionProofInputs<'a> {
    pub fn new(inputs: &'a [InclusionMerkleProofInputs]) -> Self {
        let public_input_hash = InclusionProofInputs::public_input(inputs);
        InclusionProofInputs {
            public_input_hash,
            inputs,
        }
    }
    pub fn public_input(inputs: &'a [InclusionMerkleProofInputs]) -> BigInt {
        let leaves_hash_chain = calculate_hash_chain(
            &inputs
                .iter()
                .map(|x| bigint_to_u8_32(&x.leaf).unwrap())
                .collect::<Vec<_>>(),
        );
        let roots_hash_chain = calculate_hash_chain(
            &inputs
                .iter()
                .map(|x| bigint_to_u8_32(&x.root).unwrap())
                .collect::<Vec<_>>(),
        );
        BigInt::from_bytes_be(
            num_bigint::Sign::Plus,
            &calculate_hash_chain(&[roots_hash_chain, leaves_hash_chain]),
        )
    }
}
