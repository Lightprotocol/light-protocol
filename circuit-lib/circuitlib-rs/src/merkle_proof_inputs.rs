use std::{collections::HashMap, convert::TryInto, fmt};

use ark_circom::circom::Inputs;
use num_bigint::BigInt;

#[derive(Clone, Debug)]
pub enum MerkleTreeInfo {
    H22,
}

impl MerkleTreeInfo {
    pub fn height(&self) -> u8 {
        match self {
            MerkleTreeInfo::H22 => 22,
        }
    }
    pub fn test_zk_path(&self, num_of_utxos: usize) -> String {
        format!(
            "test-data/merkle{}_{}/circuit.zkey",
            self.height(),
            num_of_utxos
        )
    }
    pub fn test_wasm_path(&self, num_of_utxos: usize) -> String {
        format!(
            "test-data/merkle{}_{}/circuit.wasm",
            self.height(),
            num_of_utxos
        )
    }
}

impl fmt::Display for MerkleTreeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.height())
    }
}

#[derive(Clone, Debug)]
pub struct MerkleTreeProofInput {
    pub root: BigInt,
    pub leaf: BigInt,
    pub in_path_indices: BigInt,
    pub in_path_elements: Vec<BigInt>,
}

impl MerkleTreeProofInput {
    pub fn public_inputs_arr(&self) -> [[u8; 32]; 2] {
        let root: [u8; 32] = <[u8; 32]>::try_from(self.root.to_bytes_be().1.as_slice()).unwrap();
        let leaf: [u8; 32] = <[u8; 32]>::try_from(self.leaf.to_bytes_be().1.as_slice()).unwrap();
        [root, leaf]
    }
}

pub fn public_inputs(merkle_proof_inputs: &[MerkleTreeProofInput]) -> Vec<[u8; 32]> {
    let mut roots = Vec::new();
    let mut leafs = Vec::new();
    for input in merkle_proof_inputs {
        let input_arr = input.public_inputs_arr();
        roots.push(input_arr[0]);
        leafs.push(input_arr[1]);
    }
    [roots, leafs].concat()
}

pub struct ProofInputs<'a>(pub &'a [MerkleTreeProofInput]);
impl<'a> TryInto<HashMap<String, Inputs>> for ProofInputs<'a> {
    type Error = std::io::Error;

    fn try_into(self) -> Result<HashMap<String, Inputs>, Self::Error> {
        let mut inputs: HashMap<String, Inputs> = HashMap::new();
        let mut roots: Vec<BigInt> = Vec::new();
        let mut leafs: Vec<BigInt> = Vec::new();
        let mut indices: Vec<BigInt> = Vec::new();
        let mut els: Vec<Vec<BigInt>> = Vec::new();

        for input in self.0 {
            roots.push(input.root.clone());
            leafs.push(input.leaf.clone());
            indices.push(input.in_path_indices.clone());
            els.push(input.in_path_elements.clone());
        }

        inputs
            .entry("root".to_string())
            .or_insert_with(|| Inputs::BigIntVec(roots));
        inputs
            .entry("leaf".to_string())
            .or_insert_with(|| Inputs::BigIntVec(leafs));
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
        let zero_input = MerkleTreeProofInput {
            leaf: BigInt::zero(),
            root: BigInt::zero(),
            in_path_elements: vec![BigInt::zero()],
            in_path_indices: BigInt::zero(),
        };

        let inputs: [MerkleTreeProofInput; 2] = [zero_input.clone(), zero_input.clone()];
        let proof_inputs = ProofInputs(&inputs);
        let result: HashMap<String, Inputs> = proof_inputs.try_into().unwrap();
        assert_eq!(result.len(), inputs.len() * 2);
    }
}
