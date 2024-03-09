use circuitlib_rs::{
    groth16_solana_verifier::groth16_solana_verify,
    helpers::init_logger,
    init_merkle_tree::non_inclusion_merkle_tree_inputs_26,
    non_inclusion::{
        merkle_non_inclusion_proof::merkle_non_inclusion_proof,
        merkle_non_inclusion_proof_inputs::NonInclusionMerkleProofInputs,
        non_inclusion_merkle_tree_info::NonInclusionMerkleTreeInfo,
    },
    verifying_keys::VK_ni_26_1,
};

#[test]
fn non_inclusion() {
    init_logger();

    let inputs: [NonInclusionMerkleProofInputs; 1] =
        std::array::from_fn(|_| non_inclusion_merkle_tree_inputs_26());
    println!("inputs: {:?}", inputs);
    println!("proving...");
    let proof_result =
        merkle_non_inclusion_proof(&NonInclusionMerkleTreeInfo::H26, &inputs).unwrap();
    println!("Proof compressed: {:?}", proof_result.proof);

    let mut public_inputs = [[0u8; 32]; 2];
    for i in 0..1 {
        public_inputs[i] = inputs[i].public_inputs_arr()[0];
        public_inputs[i + 1] = inputs[i].public_inputs_arr()[1];
    }
    println!("Public inputs: {:?}", public_inputs);
    println!("verifying...");
    let vk = VK_ni_26_1;
    let sol_verified = groth16_solana_verify(&proof_result.proof, &public_inputs, vk);
    println!("Verified: {:?}", sol_verified);
    assert!(sol_verified.is_ok());
}
