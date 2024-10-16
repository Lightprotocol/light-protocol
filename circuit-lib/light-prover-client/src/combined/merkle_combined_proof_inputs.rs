use num_bigint::BigInt;

use crate::{
    batch_append_with_subtrees::calculate_hash_chain, helpers::bigint_to_u8_32,
    inclusion::merkle_inclusion_proof_inputs::InclusionProofInputs,
    non_inclusion::merkle_non_inclusion_proof_inputs::NonInclusionProofInputs,
};

#[derive(Clone, Debug)]
pub struct CombinedProofInputs<'a> {
    pub public_input_hash: BigInt,
    pub inclusion_parameters: InclusionProofInputs<'a>,
    pub non_inclusion_parameters: NonInclusionProofInputs<'a>,
}

impl<'a> CombinedProofInputs<'a> {
    pub fn new(
        inclusion_parameters: InclusionProofInputs<'a>,
        non_inclusion_parameters: NonInclusionProofInputs<'a>,
    ) -> Self {
        let public_input_hash =
            Self::public_input(&inclusion_parameters, &non_inclusion_parameters);
        Self {
            public_input_hash,
            inclusion_parameters,
            non_inclusion_parameters,
        }
    }

    pub fn public_input(
        inclusion_parameters: &InclusionProofInputs,
        non_inclusion_parameters: &NonInclusionProofInputs,
    ) -> BigInt {
        BigInt::from_bytes_be(
            num_bigint::Sign::Plus,
            &calculate_hash_chain(&[
                bigint_to_u8_32(&inclusion_parameters.public_input_hash).unwrap(),
                bigint_to_u8_32(&non_inclusion_parameters.public_input_hash).unwrap(),
            ]),
        )
    }
}
