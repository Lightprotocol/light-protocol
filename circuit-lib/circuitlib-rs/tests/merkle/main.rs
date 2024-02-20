use circuitlib_rs::{
    groth16_solana_verifier::{groth16_solana_verify, merkle_inclusion_proof},
    helpers::init_logger,
    init_merkle_tree::merkle_tree_inputs,
    merkle_proof_inputs::{MerkleTreeInfo, MerkleTreeProofInput},
    verifying_keys::vk,
};

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

test_and_prove!(test_and_prove_26_1, MerkleTreeInfo::H26, 1);
test_and_prove!(test_and_prove_26_2, MerkleTreeInfo::H26, 2);
test_and_prove!(test_and_prove_26_3, MerkleTreeInfo::H26, 3);
test_and_prove!(test_and_prove_26_4, MerkleTreeInfo::H26, 4);
test_and_prove!(test_and_prove_26_8, MerkleTreeInfo::H26, 8);
