use account_compression::{state::QueueAccount, StateMerkleTreeAccount};
use forester_utils::account_zero_copy::{get_concurrent_merkle_tree, get_hash_set};
use light_client::rpc::Rpc;
use light_compressed_account::TreeType;
use light_hasher::Poseidon;
use light_program_test::{program_test::LightProgramTest, ProgramTestConfig};
use light_registry::account_compression_cpi::sdk::{
    compress_proofs, create_nullify_state_v1_multi_instruction, CompressedProofs,
    CreateNullifyStateV1MultiInstructionInputs,
};
use light_test_utils::e2e_test_env::init_program_test_env;
use serial_test::serial;
use solana_sdk::signature::{Keypair, Signer};

#[serial]
#[tokio::test]
async fn test_nullify_state_v1_multi_4_leaves() {
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

        for _ in 0..4 {
            e2e_env
                .compress_sol_deterministic(&forester_keypair, 1_000_000, None)
                .await;
            e2e_env
                .transfer_sol_deterministic(&forester_keypair, &Keypair::new().pubkey(), None)
                .await
                .unwrap();
        }

        (e2e_env.indexer.state_merkle_trees[0].clone(), e2e_env.rpc)
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

    // Collect 4 unmarked items
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
        items_to_nullify.len() >= 4,
        "Need at least 4 items in nullifier queue, got {}",
        items_to_nullify.len()
    );

    // Get proofs
    let mut leaf_indices = Vec::new();
    let mut proofs = Vec::new();
    for (_, leaf) in items_to_nullify.iter().take(4) {
        let leaf_index = state_tree_bundle.merkle_tree.get_leaf_index(leaf).unwrap();
        leaf_indices.push(leaf_index);
        let proof: Vec<[u8; 32]> = state_tree_bundle
            .merkle_tree
            .get_proof_of_leaf(leaf_index, false)
            .unwrap();
        let proof_arr: [[u8; 32]; 16] = proof.try_into().unwrap();
        proofs.push(proof_arr);
    }

    // Verify shared top node
    for i in 1..4 {
        assert_eq!(
            proofs[0][15], proofs[i][15],
            "Level 15 proof node must be shared between all leaves"
        );
    }

    let proof_refs: Vec<&[[u8; 32]; 16]> = proofs.iter().collect();
    let CompressedProofs {
        proof_2_shared,
        proof_3_source,
        proof_4_source,
        shared_top_node,
        nodes,
    } = compress_proofs(&proof_refs).expect("compress_proofs should succeed for 4 leaves");

    let queue_indices: [u16; 4] = [
        items_to_nullify[0].0 as u16,
        items_to_nullify[1].0 as u16,
        items_to_nullify[2].0 as u16,
        items_to_nullify[3].0 as u16,
    ];
    let leaf_indices_arr: [u32; 4] = [
        leaf_indices[0] as u32,
        leaf_indices[1] as u32,
        leaf_indices[2] as u32,
        leaf_indices[3] as u32,
    ];

    let ix = create_nullify_state_v1_multi_instruction(
        CreateNullifyStateV1MultiInstructionInputs {
            authority: forester_keypair.pubkey(),
            nullifier_queue: state_tree_bundle.accounts.nullifier_queue,
            merkle_tree: state_tree_bundle.accounts.merkle_tree,
            change_log_index: change_log_index as u16,
            queue_indices,
            leaf_indices: leaf_indices_arr,
            proof_2_shared,
            proof_3_source,
            proof_4_source,
            shared_top_node,
            nodes,
            derivation: forester_keypair.pubkey(),
            is_metadata_forester: true,
        },
        0,
    );

    rpc.create_and_send_transaction(&[ix], &forester_keypair.pubkey(), &[&forester_keypair])
        .await
        .unwrap();

    // Verify all 4 queue items marked
    let nullifier_queue_post = unsafe {
        get_hash_set::<QueueAccount, _>(&mut rpc, state_tree_bundle.accounts.nullifier_queue)
            .await
            .unwrap()
    };
    for (idx, (queue_idx, _)) in items_to_nullify.iter().take(4).enumerate() {
        let bucket = nullifier_queue_post
            .get_bucket(*queue_idx)
            .unwrap()
            .unwrap();
        assert!(
            bucket.sequence_number.is_some(),
            "Queue item {} should be marked after nullify_state_v1_multi",
            idx
        );
    }

    // Verify root changed
    let onchain_tree_post = get_concurrent_merkle_tree::<StateMerkleTreeAccount, _, Poseidon, 26>(
        &mut rpc,
        state_tree_bundle.accounts.merkle_tree,
    )
    .await
    .unwrap();
    assert_ne!(
        pre_root,
        onchain_tree_post.root(),
        "Root should have changed after nullify_state_v1_multi"
    );

    // Locally update and verify root match
    for &li in &leaf_indices {
        state_tree_bundle
            .merkle_tree
            .update(&[0u8; 32], li)
            .unwrap();
    }
    assert_eq!(
        onchain_tree_post.root(),
        state_tree_bundle.merkle_tree.root(),
        "On-chain root should match local tree after nullifying all 4 leaves"
    );
}

#[serial]
#[tokio::test]
async fn test_nullify_state_v1_multi_3_leaves() {
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

        for _ in 0..3 {
            e2e_env
                .compress_sol_deterministic(&forester_keypair, 1_000_000, None)
                .await;
            e2e_env
                .transfer_sol_deterministic(&forester_keypair, &Keypair::new().pubkey(), None)
                .await
                .unwrap();
        }

        (e2e_env.indexer.state_merkle_trees[0].clone(), e2e_env.rpc)
    };

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
    let change_log_index = onchain_tree.changelog_index();

    let mut items_to_nullify = Vec::new();
    for i in 0..nullifier_queue.get_capacity() {
        let bucket = nullifier_queue.get_bucket(i).unwrap();
        if let Some(bucket) = bucket {
            if bucket.sequence_number.is_none() {
                items_to_nullify.push((i, bucket.value_bytes()));
            }
        }
    }
    assert!(items_to_nullify.len() >= 3);

    let mut leaf_indices = Vec::new();
    let mut proofs = Vec::new();
    for (_, leaf) in items_to_nullify.iter().take(3) {
        let leaf_index = state_tree_bundle.merkle_tree.get_leaf_index(leaf).unwrap();
        leaf_indices.push(leaf_index);
        let proof: Vec<[u8; 32]> = state_tree_bundle
            .merkle_tree
            .get_proof_of_leaf(leaf_index, false)
            .unwrap();
        proofs.push(<[[u8; 32]; 16]>::try_from(proof).unwrap());
    }

    let proof_refs: Vec<&[[u8; 32]; 16]> = proofs.iter().collect();
    let CompressedProofs {
        proof_2_shared,
        proof_3_source,
        proof_4_source,
        shared_top_node,
        nodes,
    } = compress_proofs(&proof_refs).expect("compress_proofs should succeed for 3 leaves");

    let ix = create_nullify_state_v1_multi_instruction(
        CreateNullifyStateV1MultiInstructionInputs {
            authority: forester_keypair.pubkey(),
            nullifier_queue: state_tree_bundle.accounts.nullifier_queue,
            merkle_tree: state_tree_bundle.accounts.merkle_tree,
            change_log_index: change_log_index as u16,
            queue_indices: [
                items_to_nullify[0].0 as u16,
                items_to_nullify[1].0 as u16,
                items_to_nullify[2].0 as u16,
                0,
            ],
            leaf_indices: [
                leaf_indices[0] as u32,
                leaf_indices[1] as u32,
                leaf_indices[2] as u32,
                u32::MAX,
            ],
            proof_2_shared,
            proof_3_source,
            proof_4_source,
            shared_top_node,
            nodes,
            derivation: forester_keypair.pubkey(),
            is_metadata_forester: true,
        },
        0,
    );

    rpc.create_and_send_transaction(&[ix], &forester_keypair.pubkey(), &[&forester_keypair])
        .await
        .unwrap();

    // Verify 3 queue items marked
    let nullifier_queue_post = unsafe {
        get_hash_set::<QueueAccount, _>(&mut rpc, state_tree_bundle.accounts.nullifier_queue)
            .await
            .unwrap()
    };
    for (idx, (queue_idx, _)) in items_to_nullify.iter().take(3).enumerate() {
        let bucket = nullifier_queue_post
            .get_bucket(*queue_idx)
            .unwrap()
            .unwrap();
        assert!(
            bucket.sequence_number.is_some(),
            "Queue item {} should be marked",
            idx
        );
    }

    // Locally update and verify root match
    for &li in &leaf_indices {
        state_tree_bundle
            .merkle_tree
            .update(&[0u8; 32], li)
            .unwrap();
    }
    let onchain_tree_post = get_concurrent_merkle_tree::<StateMerkleTreeAccount, _, Poseidon, 26>(
        &mut rpc,
        state_tree_bundle.accounts.merkle_tree,
    )
    .await
    .unwrap();
    assert_eq!(
        onchain_tree_post.root(),
        state_tree_bundle.merkle_tree.root(),
    );
}

#[serial]
#[tokio::test]
async fn test_nullify_state_v1_multi_2_leaves() {
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

        for _ in 0..2 {
            e2e_env
                .compress_sol_deterministic(&forester_keypair, 1_000_000, None)
                .await;
            e2e_env
                .transfer_sol_deterministic(&forester_keypair, &Keypair::new().pubkey(), None)
                .await
                .unwrap();
        }

        (e2e_env.indexer.state_merkle_trees[0].clone(), e2e_env.rpc)
    };

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
    let change_log_index = onchain_tree.changelog_index();

    let mut items_to_nullify = Vec::new();
    for i in 0..nullifier_queue.get_capacity() {
        let bucket = nullifier_queue.get_bucket(i).unwrap();
        if let Some(bucket) = bucket {
            if bucket.sequence_number.is_none() {
                items_to_nullify.push((i, bucket.value_bytes()));
            }
        }
    }
    assert!(items_to_nullify.len() >= 2);

    let mut leaf_indices = Vec::new();
    let mut proofs = Vec::new();
    for (_, leaf) in items_to_nullify.iter().take(2) {
        let leaf_index = state_tree_bundle.merkle_tree.get_leaf_index(leaf).unwrap();
        leaf_indices.push(leaf_index);
        let proof: Vec<[u8; 32]> = state_tree_bundle
            .merkle_tree
            .get_proof_of_leaf(leaf_index, false)
            .unwrap();
        proofs.push(<[[u8; 32]; 16]>::try_from(proof).unwrap());
    }

    let proof_refs: Vec<&[[u8; 32]; 16]> = proofs.iter().collect();
    let CompressedProofs {
        proof_2_shared,
        proof_3_source,
        proof_4_source,
        shared_top_node,
        nodes,
    } = compress_proofs(&proof_refs).expect("compress_proofs should succeed for 2 leaves");

    let ix = create_nullify_state_v1_multi_instruction(
        CreateNullifyStateV1MultiInstructionInputs {
            authority: forester_keypair.pubkey(),
            nullifier_queue: state_tree_bundle.accounts.nullifier_queue,
            merkle_tree: state_tree_bundle.accounts.merkle_tree,
            change_log_index: change_log_index as u16,
            queue_indices: [
                items_to_nullify[0].0 as u16,
                items_to_nullify[1].0 as u16,
                0,
                0,
            ],
            leaf_indices: [
                leaf_indices[0] as u32,
                leaf_indices[1] as u32,
                u32::MAX,
                u32::MAX,
            ],
            proof_2_shared,
            proof_3_source,
            proof_4_source,
            shared_top_node,
            nodes,
            derivation: forester_keypair.pubkey(),
            is_metadata_forester: true,
        },
        0,
    );

    rpc.create_and_send_transaction(&[ix], &forester_keypair.pubkey(), &[&forester_keypair])
        .await
        .unwrap();

    // Verify 2 queue items marked
    let nullifier_queue_post = unsafe {
        get_hash_set::<QueueAccount, _>(&mut rpc, state_tree_bundle.accounts.nullifier_queue)
            .await
            .unwrap()
    };
    for (idx, (queue_idx, _)) in items_to_nullify.iter().take(2).enumerate() {
        let bucket = nullifier_queue_post
            .get_bucket(*queue_idx)
            .unwrap()
            .unwrap();
        assert!(
            bucket.sequence_number.is_some(),
            "Queue item {} should be marked",
            idx
        );
    }

    // Locally update and verify root match
    for &li in &leaf_indices {
        state_tree_bundle
            .merkle_tree
            .update(&[0u8; 32], li)
            .unwrap();
    }
    let onchain_tree_post = get_concurrent_merkle_tree::<StateMerkleTreeAccount, _, Poseidon, 26>(
        &mut rpc,
        state_tree_bundle.accounts.merkle_tree,
    )
    .await
    .unwrap();
    assert_eq!(
        onchain_tree_post.root(),
        state_tree_bundle.merkle_tree.root(),
    );
}

#[serial]
#[tokio::test]
async fn test_nullify_state_v1_multi_1_leaf_fails() {
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

    let (state_tree_bundle, mut rpc) = {
        let mut e2e_env = init_program_test_env(rpc, &env, 50).await;
        e2e_env.indexer.state_merkle_trees.clear();
        e2e_env.keypair_action_config.fee_assert = false;

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

        e2e_env
            .compress_sol_deterministic(&forester_keypair, 1_000_000, None)
            .await;
        e2e_env
            .transfer_sol_deterministic(&forester_keypair, &Keypair::new().pubkey(), None)
            .await
            .unwrap();

        (e2e_env.indexer.state_merkle_trees[0].clone(), e2e_env.rpc)
    };

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
    let change_log_index = onchain_tree.changelog_index();

    let mut items_to_nullify = Vec::new();
    for i in 0..nullifier_queue.get_capacity() {
        let bucket = nullifier_queue.get_bucket(i).unwrap();
        if let Some(bucket) = bucket {
            if bucket.sequence_number.is_none() {
                items_to_nullify.push((i, bucket.value_bytes()));
            }
        }
    }
    assert!(!items_to_nullify.is_empty());

    let leaf_index = state_tree_bundle
        .merkle_tree
        .get_leaf_index(&items_to_nullify[0].1)
        .unwrap();
    let proof: Vec<[u8; 32]> = state_tree_bundle
        .merkle_tree
        .get_proof_of_leaf(leaf_index, false)
        .unwrap();
    let proof_arr: [[u8; 32]; 16] = proof.try_into().unwrap();

    let nodes: Vec<[u8; 32]> = proof_arr[..15].to_vec();
    let shared_top_node = proof_arr[15];

    let ix = create_nullify_state_v1_multi_instruction(
        CreateNullifyStateV1MultiInstructionInputs {
            authority: forester_keypair.pubkey(),
            nullifier_queue: state_tree_bundle.accounts.nullifier_queue,
            merkle_tree: state_tree_bundle.accounts.merkle_tree,
            change_log_index: change_log_index as u16,
            queue_indices: [items_to_nullify[0].0 as u16, 0, 0, 0],
            leaf_indices: [leaf_index as u32, u32::MAX, u32::MAX, u32::MAX],
            proof_2_shared: 0,
            proof_3_source: 0,
            proof_4_source: 0,
            shared_top_node,
            nodes,
            derivation: forester_keypair.pubkey(),
            is_metadata_forester: true,
        },
        0,
    );

    let result = rpc
        .create_and_send_transaction(&[ix], &forester_keypair.pubkey(), &[&forester_keypair])
        .await;

    assert!(
        result.is_err(),
        "nullify_state_v1_multi with 1 leaf should fail with InvalidProofEncoding"
    );
}
