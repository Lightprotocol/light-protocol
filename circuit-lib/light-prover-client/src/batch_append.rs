use crate::helpers::bigint_to_u8_32;
use num_bigint::BigInt;

#[derive(Clone, Debug, Default)]
pub struct BatchAppendCircuitInputs {
    pub public_input_hash: BigInt,
    pub old_sub_tree_hash_chain: BigInt,
    pub new_sub_tree_hash_chain: BigInt,
    pub new_root: BigInt,
    pub hashchain_hash: BigInt,
    pub start_index: BigInt,
    pub hash_chain_start_index: BigInt,
    pub tree_height: BigInt,
    pub leaves: Vec<BigInt>,
    pub subtrees: Vec<BigInt>,
}

impl BatchAppendCircuitInputs {
    pub fn public_inputs_arr(&self) -> [u8; 32] {
        bigint_to_u8_32(&self.public_input_hash).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct BatchAppendInputs<'a>(pub &'a [BatchAppendCircuitInputs]);

impl BatchAppendInputs<'_> {
    pub fn public_inputs(&self) -> Vec<[u8; 32]> {
        // Concatenate all public inputs into a single flat vector
        vec![self.0[0].public_inputs_arr()]
    }
}
