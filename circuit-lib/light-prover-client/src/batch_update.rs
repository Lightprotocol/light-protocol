use crate::helpers::bigint_to_u8_32;
use num_bigint::BigInt;

#[derive(Clone, Debug)]
pub struct BatchUpdateCircuitInputs {
    pub public_input_hash: BigInt,
    pub old_root: BigInt,
    pub new_root: BigInt,
    pub leaves_hashchain_hash: BigInt,
    pub leaves: Vec<BigInt>,
    pub merkle_proofs: Vec<Vec<BigInt>>,
    pub path_indices: Vec<u32>,
    pub height: u32,
    pub batch_size: u32,
}

impl BatchUpdateCircuitInputs {
    pub fn public_inputs_arr(&self) -> [u8; 32] {
        bigint_to_u8_32(&self.public_input_hash).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct BatchUpdateInputs<'a>(pub &'a [BatchUpdateCircuitInputs]);

impl BatchUpdateInputs<'_> {
    pub fn public_inputs(&self) -> Vec<[u8; 32]> {
        // Concatenate all public inputs into a single flat vector
        vec![self.0[0].public_inputs_arr()]
    }
}
