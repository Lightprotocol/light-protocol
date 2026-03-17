use account_compression::{state::QueueAccount, StateMerkleTreeAccount};
use forester_utils::account_zero_copy::{get_concurrent_merkle_tree, get_hash_set};
use light_client::rpc::Rpc;
use light_compressed_account::TreeType;
use light_hasher::Poseidon;
use light_program_test::{program_test::LightProgramTest, ProgramTestConfig};
use light_registry::account_compression_cpi::sdk::{
    create_nullify_2_instruction, CreateNullify2InstructionInputs,
};
use light_test_utils::e2e_test_env::init_program_test_env;
use serial_test::serial;
use solana_sdk::signature::{Keypair, Signer};

/// Tests that nullify_2 correctly nullifies two leaves in a single instruction
/// using two sequential CPIs to account_compression::nullify_leaves.
/// Uses LiteSVM (light-program-test) for fast logic testing.
/// Note: LiteSVM allows 10KB transactions, so this does NOT validate tx size.
#[serial]
#[tokio::test]
async fn test_nullify_2() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::default_with_batched_trees(true))
        .await
        .unwrap();
    rpc.indexer = None;

    let env = rpc.test_accounts.clone();

    let forester_keypair = Keypair::new();
    rpc.airdrop_lamports(&forester_keypair.pubkey(), 2_000_000_000)
        .await
        .unwrap();

    let merkle_tree_keypair = Keypair::new();
    let nullifier_queue_keypair = Keypair::new();
    let cpi_context_keypair = Keypair::new();

    let (mut state_tree_bundle, mut rpc) = {
        let mut e2e_env = init_program_test_env(rpc, &env, 50).await;
        e2e_env.indexer.state_merkle_trees.clear();
        e2e_env.keypair_action_config.fee_assert = false;

        // Create V1 state merkle tree with custom forester
        e2e_env
            .indexer
            .add_state_merkle_tree(
                &mut e2e_env.rpc,
                &merkle_tree_keypair,
                &nullifier_queue_keypair,
                &cpi_context_keypair,
                None,
                Some(forester_keypair.pubkey()),
                TreeType::StateV1,
            )
            .await;

        // Create 2 compressed accounts by compressing + transferring twice.
        // Each transfer nullifies the input, putting it in the nullifier queue.
        e2e_env
            .compress_sol_deterministic(&forester_keypair, 1_000_000, None)
            .await;
        e2e_env
            .transfer_sol_deterministic(
                &forester_keypair,
                &Keypair::new().pubkey(),
                None,
            )
            .await
            .unwrap();

        e2e_env
            .compress_sol_deterministic(&forester_keypair, 1_000_000, None)
            .await;
        e2e_env
            .transfer_sol_deterministic(
                &forester_keypair,
                &Keypair::new().pubkey(),
                None,
            )
            .await
            .unwrap();

        (
            e2e_env.indexer.state_merkle_trees[0].clone(),
            e2e_env.rpc,
        )
    };

    // Read on-chain state
    let nullifier_queue = unsafe {
        get_hash_set::<QueueAccount, _>(&mut rpc, state_tree_bundle.accounts.nullifier_queue)
            .await
            .unwrap()
    };

    let onchain_tree = get_concurrent_merkle_tree::<StateMerkleTreeAccount, _, Poseidon, 26>(
        &mut rpc,
        state_tree_bundle.accounts.merkle_tree,
    )
    .await
    .unwrap();

    let pre_root = onchain_tree.root();
    let change_log_index = onchain_tree.changelog_index();

    // Collect 2 unmarked items from the queue
    let mut items_to_nullify = Vec::new();
    for i in 0..nullifier_queue.get_capacity() {
        let bucket = nullifier_queue.get_bucket(i).unwrap();
        if let Some(bucket) = bucket {
            if bucket.sequence_number.is_none() {
                items_to_nullify.push((i, bucket.value_bytes()));
            }
        }
    }
    assert!(
        items_to_nullify.len() >= 2,
        "Need at least 2 items in nullifier queue, got {}",
        items_to_nullify.len()
    );

    let (queue_idx_0, leaf_0) = items_to_nullify[0];
    let (queue_idx_1, leaf_1) = items_to_nullify[1];

    let leaf_index_0 = state_tree_bundle
        .merkle_tree
        .get_leaf_index(&leaf_0)
        .unwrap();
    let leaf_index_1 = state_tree_bundle
        .merkle_tree
        .get_leaf_index(&leaf_1)
        .unwrap();

    let proof_0: Vec<[u8; 32]> = state_tree_bundle
        .merkle_tree
        .get_proof_of_leaf(leaf_index_0, false)
        .unwrap();
    let proof_1: Vec<[u8; 32]> = state_tree_bundle
        .merkle_tree
        .get_proof_of_leaf(leaf_index_1, false)
        .unwrap();

    // Split proofs: first 15 nodes are unique per leaf, node at index 15 is shared.
    // Both leaves are in the same 2^16 subtree so they share the proof node at level 15.
    let proof_0_arr: [[u8; 32]; 15] = proof_0[..15].try_into().unwrap();
    let proof_1_arr: [[u8; 32]; 15] = proof_1[..15].try_into().unwrap();
    let shared_proof_node: [u8; 32] = proof_0[15];
    // Verify the shared node is the same in both proofs.
    assert_eq!(
        proof_0[15], proof_1[15],
        "Level 15 proof node must be shared between both leaves in the same subtree"
    );

    // Build nullify_2 instruction
    let ix = create_nullify_2_instruction(
        CreateNullify2InstructionInputs {
            authority: forester_keypair.pubkey(),
            nullifier_queue: state_tree_bundle.accounts.nullifier_queue,
            merkle_tree: state_tree_bundle.accounts.merkle_tree,
            change_log_index: change_log_index as u16,
            queue_index_0: queue_idx_0 as u16,
            queue_index_1: queue_idx_1 as u16,
            leaf_index_0: leaf_index_0 as u32,
            leaf_index_1: leaf_index_1 as u32,
            proof_0: proof_0_arr,
            proof_1: proof_1_arr,
            shared_proof_node,
            derivation: forester_keypair.pubkey(),
            is_metadata_forester: true,
        },
        0,
    );

    // Send transaction
    rpc.create_and_send_transaction(&[ix], &forester_keypair.pubkey(), &[&forester_keypair])
        .await
        .unwrap();

    // Verify: both queue items should be marked
    let nullifier_queue_post = unsafe {
        get_hash_set::<QueueAccount, _>(&mut rpc, state_tree_bundle.accounts.nullifier_queue)
            .await
            .unwrap()
    };

    let bucket_0 = nullifier_queue_post
        .get_bucket(queue_idx_0)
        .unwrap()
        .unwrap();
    assert!(
        bucket_0.sequence_number.is_some(),
        "First queue item should be marked after nullify_2"
    );

    let bucket_1 = nullifier_queue_post
        .get_bucket(queue_idx_1)
        .unwrap()
        .unwrap();
    assert!(
        bucket_1.sequence_number.is_some(),
        "Second queue item should be marked after nullify_2"
    );

    // Verify: tree root changed
    let onchain_tree_post =
        get_concurrent_merkle_tree::<StateMerkleTreeAccount, _, Poseidon, 26>(
            &mut rpc,
            state_tree_bundle.accounts.merkle_tree,
        )
        .await
        .unwrap();
    assert_ne!(
        pre_root,
        onchain_tree_post.root(),
        "Root should have changed after nullify_2"
    );

    // Locally update the merkle tree and verify roots match
    state_tree_bundle
        .merkle_tree
        .update(&[0u8; 32], leaf_index_0)
        .unwrap();
    state_tree_bundle
        .merkle_tree
        .update(&[0u8; 32], leaf_index_1)
        .unwrap();

    assert_eq!(
        onchain_tree_post.root(),
        state_tree_bundle.merkle_tree.root(),
        "On-chain root should match local tree after nullifying both leaves"
    );
}
