use circuitlib_rs::{
    groth16_solana_verifier::{groth16_solana_verify, merkle_inclusion_proof},
    init_merkle_tree::merkle_tree_inputs,
    merkle_proof_inputs::{MerkleTreeInfo, MerkleTreeProofInput},
    verifying_keys::vk,
};
use env_logger::Builder;
use log::LevelFilter;

macro_rules! test_and_prove {
    ($fn_name:ident, $mt_height:expr, $nr_inputs:expr) => {
        #[test]
        fn $fn_name() {
            init_logger();
            let inputs: [MerkleTreeProofInput; $nr_inputs] =
                std::array::from_fn(|_| merkle_tree_inputs($mt_height));
            let proof_result = merkle_inclusion_proof(&$mt_height, &inputs).unwrap();
            let mut public_inputs = [[0u8; 32]; $nr_inputs * 2];
            for i in 0..$nr_inputs {
                public_inputs[i] = inputs[i].public_inputs_arr()[0];
                public_inputs[i + $nr_inputs] = inputs[i].public_inputs_arr()[1];
            }
            let vk = vk($mt_height, $nr_inputs).unwrap();
            let sol_verified =
                groth16_solana_verify(&proof_result.proof, &public_inputs, *vk).unwrap();
            assert!(sol_verified);
        }
    };
}

test_and_prove!(test_and_prove_22_1, MerkleTreeInfo::H22, 1);
test_and_prove!(test_and_prove_22_2, MerkleTreeInfo::H22, 2);
test_and_prove!(test_and_prove_22_3, MerkleTreeInfo::H22, 3);
// test_and_prove!(test_and_prove_22_4, MerkleTreeInfo::H22, 4);
// test_and_prove!(test_and_prove_22_5, MerkleTreeInfo::H22, 5);
// test_and_prove!(test_and_prove_22_6, MerkleTreeInfo::H22, 6);
// test_and_prove!(test_and_prove_22_7, MerkleTreeInfo::H22, 7);
// test_and_prove!(test_and_prove_22_8, MerkleTreeInfo::H22, 8);
// test_and_prove!(test_and_prove_22_9, MerkleTreeInfo::H22, 9);
// test_and_prove!(test_and_prove_22_10, MerkleTreeInfo::H22, 10);
//
// test_and_prove!(test_and_prove_30_1, MerkleTreeInfo::H30, 1);
// test_and_prove!(test_and_prove_30_2, MerkleTreeInfo::H30, 2);
// test_and_prove!(test_and_prove_30_3, MerkleTreeInfo::H30, 3);
// test_and_prove!(test_and_prove_30_4, MerkleTreeInfo::H30, 4);
// test_and_prove!(test_and_prove_30_5, MerkleTreeInfo::H30, 5);
// test_and_prove!(test_and_prove_30_6, MerkleTreeInfo::H30, 6);
// test_and_prove!(test_and_prove_30_7, MerkleTreeInfo::H30, 7);
// test_and_prove!(test_and_prove_30_8, MerkleTreeInfo::H30, 8);
// test_and_prove!(test_and_prove_30_9, MerkleTreeInfo::H30, 9);
// test_and_prove!(test_and_prove_30_10, MerkleTreeInfo::H30, 10);

fn init_logger() {
    let _ = Builder::new()
        .filter_module("circuitlib_rs", LevelFilter::Info)
        .try_init();
}
