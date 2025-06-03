use light_hasher::hash_chain::create_two_inputs_hash_chain;
use num_bigint::BigInt;

use crate::{errors::ProverClientError, helpers::bigint_to_u8_32};

#[derive(Clone, Debug)]
pub struct NonInclusionMerkleProofInputs {
    pub root: BigInt,
    pub value: BigInt,

    pub leaf_lower_range_value: BigInt,
    pub leaf_higher_range_value: BigInt,

    pub merkle_proof_hashed_indexed_element_leaf: Vec<BigInt>,
    pub index_hashed_indexed_element_leaf: BigInt,
    pub next_index: BigInt,
}

#[derive(Clone, Debug)]
pub struct NonInclusionProofInputs<'a> {
    pub public_input_hash: BigInt,
    pub inputs: &'a [NonInclusionMerkleProofInputs],
}

impl<'a> NonInclusionProofInputs<'a> {
    pub fn new(inputs: &'a [NonInclusionMerkleProofInputs]) -> Result<Self, ProverClientError> {
        let public_input_hash = Self::public_input(inputs)?;
        Ok(Self {
            public_input_hash,
            inputs,
        })
    }

    pub fn public_input(
        inputs: &'a [NonInclusionMerkleProofInputs],
    ) -> Result<BigInt, ProverClientError> {
        let public_input_hash = create_two_inputs_hash_chain(
            &inputs
                .iter()
                .map(|x| bigint_to_u8_32(&x.root).unwrap())
                .collect::<Vec<_>>(),
            &inputs
                .iter()
                .map(|x| bigint_to_u8_32(&x.value).unwrap())
                .collect::<Vec<_>>(),
        )?;
        Ok(BigInt::from_bytes_be(
            num_bigint::Sign::Plus,
            &public_input_hash,
        ))
    }
}
