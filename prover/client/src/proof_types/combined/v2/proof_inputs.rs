use light_hasher::hash_chain::create_hash_chain_from_array;
use num_bigint::BigInt;

use crate::{
    errors::ProverClientError,
    helpers::bigint_to_u8_32,
    proof_types::{
        inclusion::v2::InclusionProofInputs, non_inclusion::v2::NonInclusionProofInputs,
    },
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
    ) -> Result<Self, ProverClientError> {
        let public_input_hash =
            Self::public_input(&inclusion_parameters, &non_inclusion_parameters)?;
        Ok(Self {
            public_input_hash,
            inclusion_parameters,
            non_inclusion_parameters,
        })
    }

    pub fn public_input(
        inclusion_parameters: &InclusionProofInputs,
        non_inclusion_parameters: &NonInclusionProofInputs,
    ) -> Result<BigInt, ProverClientError> {
        Ok(BigInt::from_bytes_be(
            num_bigint::Sign::Plus,
            &create_hash_chain_from_array([
                bigint_to_u8_32(&inclusion_parameters.public_input_hash).unwrap(),
                bigint_to_u8_32(&non_inclusion_parameters.public_input_hash).unwrap(),
            ])?,
        ))
    }
}
