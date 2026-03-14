use light_hasher::hash_chain::create_two_inputs_hash_chain;
use num_bigint::BigInt;

use crate::{errors::ProverClientError, helpers::bigint_to_u8_32};

#[derive(Clone, Debug)]
pub struct InclusionMerkleProofInputs {
    pub root: BigInt,
    pub leaf: BigInt,
    pub path_index: BigInt,
    pub path_elements: Vec<BigInt>,
}

#[derive(Clone, Debug)]
pub struct InclusionProofInputs<'a> {
    pub public_input_hash: BigInt,
    pub inputs: &'a [InclusionMerkleProofInputs],
}

impl<'a> InclusionProofInputs<'a> {
    pub fn new(inputs: &'a [InclusionMerkleProofInputs]) -> Result<Self, ProverClientError> {
        let public_input_hash = InclusionProofInputs::public_input(inputs)?;
        Ok(InclusionProofInputs {
            public_input_hash,
            inputs,
        })
    }
    pub fn public_input(
        inputs: &'a [InclusionMerkleProofInputs],
    ) -> Result<BigInt, ProverClientError> {
        let mut roots = Vec::with_capacity(inputs.len());
        let mut leaves = Vec::with_capacity(inputs.len());
        for input in inputs {
            roots.push(bigint_to_u8_32(&input.root)?);
            leaves.push(bigint_to_u8_32(&input.leaf)?);
        }
        let public_input_hash = create_two_inputs_hash_chain(&roots, &leaves)?;
        Ok(BigInt::from_bytes_be(
            num_bigint::Sign::Plus,
            &public_input_hash,
        ))
    }
}
