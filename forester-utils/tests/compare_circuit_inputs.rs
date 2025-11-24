/// Test to compare circuit inputs between old (v2 changelog) and new (v3 staging tree) approaches
/// Run forester with DUMP_PHOTON_DATA=1 to capture real data, then run this test
use light_batched_merkle_tree::constants::DEFAULT_BATCH_STATE_TREE_HEIGHT;
use light_hasher::{hash_chain::create_hash_chain_from_slice, Hasher, Poseidon};
use light_prover_client::proof_types::batch_append::{get_batch_append_inputs_v2, BatchAppendsCircuitInputs};
use light_prover_client::proof_types::batch_update::{get_batch_update_inputs, get_batch_update_inputs_v2};
use forester_utils::staging_tree::StagingTree;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
struct PhotonNullifyData {
    old_root: [u8; 32],
    tx_hashes: Vec<[u8; 32]>,
    account_hashes: Vec<[u8; 32]>,
    leaves_hashchain: [u8; 32],
    old_leaves: Vec<[u8; 32]>,
    merkle_proofs: Vec<Vec<[u8; 32]>>,
    path_indices: Vec<u32>,
    batch_size: u32,
    leaf_indices: Vec<u64>,
    new_root: [u8; 32],
}

#[derive(Debug, Serialize, Deserialize)]
struct PhotonAppendData {
    old_root: [u8; 32],
    start_index: u32,
    leaves: Vec<[u8; 32]>,
    leaves_hashchain: [u8; 32],
    old_leaves: Vec<[u8; 32]>,
    merkle_proofs: Vec<Vec<[u8; 32]>>,
    batch_size: u32,
    leaf_indices: Vec<u64>,
    new_root: [u8; 32],
}

#[test]
#[ignore] // Run with: cargo test compare_nullify_old_vs_new -- --ignored --nocapture
fn compare_nullify_old_vs_new() {
    println!("\n=== Comparing OLD vs NEW NULLIFY Circuit Inputs ===\n");

    // Load dumped data from forester run (run with DUMP_PHOTON_DATA=1)
    let data_path = "/tmp/photon_nullify_batch0.json";
    if !std::path::Path::new(data_path).exists() {
        println!("⚠️  Data file not found: {}", data_path);
        println!("   Run forester with: DUMP_PHOTON_DATA=1 cargo test ...");
        return;
    }

    let json_data = fs::read_to_string(data_path).expect("Failed to read data file");
    let data: PhotonNullifyData = serde_json::from_str(&json_data).expect("Failed to parse JSON");

    println!("📊 Test data loaded:");
    println!("   Batch size: {}", data.batch_size);
    println!("   Old root: {:?}[..4]", &data.old_root[..4]);
    println!("   New root (staging): {:?}[..4]", &data.new_root[..4]);
    println!("   Account hashes: {}", data.account_hashes.len());
    println!("   Leaf indices: {:?}", data.leaf_indices);

    // === OLD METHOD (v2 with changelogs) ===
    println!("\n🔧 OLD METHOD: get_batch_update_inputs (with changelogs)");
    let old_result = get_batch_update_inputs::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
        data.old_root,
        data.tx_hashes.clone(),
        data.account_hashes.clone(),  // OLD also expects account_hashes!
        data.leaves_hashchain,
        data.old_leaves.clone(),
        data.merkle_proofs.clone(),
        data.path_indices.clone(),
        data.batch_size,
        &[], // No previous changelogs
    );

    match old_result {
        Ok((old_inputs, old_changelog)) => {
            let old_new_root_bytes = old_inputs.new_root.to_bytes_be().1;
            println!("✅ OLD succeeded");
            println!("   Old root: {}", old_inputs.old_root);
            println!("   New root: {}", old_inputs.new_root);
            println!("   New root bytes: {:?}[..4]", &old_new_root_bytes[..4.min(old_new_root_bytes.len())]);
            println!("   Changelog entries: {}", old_changelog.len());

            // === NEW METHOD (v3 with staging tree) ===
            println!("\n🔧 NEW METHOD: staging tree + get_batch_update_inputs_v2");

            // NEW method passes the staging tree's computed new_root
            let new_inputs_result = get_batch_update_inputs_v2::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
                data.old_root,
                data.tx_hashes.clone(),
                data.account_hashes.clone(),
                data.leaves_hashchain,
                data.old_leaves.clone(),
                data.merkle_proofs.clone(),
                data.path_indices.clone(),
                data.batch_size,
                data.new_root,  // Using staging tree's new_root
            );

            match new_inputs_result {
                Ok(new_inputs) => {
                    let new_new_root_bytes = new_inputs.new_root.to_bytes_be().1;
                    println!("✅ NEW succeeded");
                    println!("   Old root: {}", new_inputs.old_root);
                    println!("   New root: {}", new_inputs.new_root);
                    println!("   New root bytes: {:?}[..4]", &new_new_root_bytes[..4.min(new_new_root_bytes.len())]);

                    // === COMPARISON ===
                    println!("\n📋 COMPARISON:");
                    println!("   OLD new_root: {}", old_inputs.new_root);
                    println!("   NEW new_root: {}", new_inputs.new_root);

                    if old_inputs.new_root == new_inputs.new_root {
                        println!("   ✅ NEW ROOTS MATCH!");
                    } else {
                        println!("   ❌ NEW ROOTS DIFFER!");
                        println!("   Difference found - this is the bug!");
                    }

                    println!("\n   Leaves comparison (first 3):");
                    for i in 0..3.min(data.batch_size as usize) {
                        let old_leaf_bytes = old_inputs.leaves[i].to_bytes_be().1;
                        let new_leaf_bytes = new_inputs.leaves[i].to_bytes_be().1;
                        println!("     [{}] OLD: {:?}[..4] vs NEW: {:?}[..4] {}",
                            i,
                            &old_leaf_bytes[..4.min(old_leaf_bytes.len())],
                            &new_leaf_bytes[..4.min(new_leaf_bytes.len())],
                            if old_inputs.leaves[i] == new_inputs.leaves[i] { "✓" } else { "✗" }
                        );
                    }
                }
                Err(e) => {
                    eprintln!("❌ NEW method failed: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("❌ OLD method failed: {}", e);
            panic!("Old method should work with real data!");
        }
    }

    println!("\n=== Test Complete ===\n");
}

#[test]
#[ignore] // Run with: cargo test --test compare_circuit_inputs -- --ignored --nocapture
fn test_compare_v2_vs_v3_circuit_inputs() {
    // This test requires:
    // 1. Photon DB at /var/folders/dy/l3xnfd3d6439fbs7dd_2l13h0000gn/T/photon_indexer.db
    // 2. Real test data in the DB from a forester test run

    println!("\n=== Comparing V2 (changelog) vs V3 (dedup nodes) Circuit Inputs ===\n");

    // TODO: Query real data from Photon DB
    // For now, let's create a minimal test case to demonstrate the comparison

    // Sample test data (replace with real Photon data)
    let initial_root = [0u8; 32];
    let start_index = 0u32;
    let zkp_batch_size = 10u32;

    // Mock data - in real test, query from DB
    let leaves = vec![[1u8; 32]; zkp_batch_size as usize];
    let old_leaves = vec![[0u8; 32]; zkp_batch_size as usize];
    let leaf_indices: Vec<u64> = (0..zkp_batch_size as u64).collect();

    // Generate merkle proofs (simplified - real test should get from Photon)
    let merkle_proofs: Vec<Vec<[u8; 32]>> = (0..zkp_batch_size)
        .map(|_| vec![[0u8; 32]; DEFAULT_BATCH_STATE_TREE_HEIGHT as usize])
        .collect();

    let leaves_hashchain = create_hash_chain_from_slice(&leaves).unwrap();

    // V3 approach: Build circuit inputs using StagingTree with dedup nodes
    println!("Building V3 circuit inputs (dedup nodes)...");
    let v3_result = build_v3_circuit_inputs(
        initial_root,
        start_index,
        &leaves,
        leaves_hashchain,
        &old_leaves,
        &merkle_proofs,
        zkp_batch_size,
        &leaf_indices,
    );

    match v3_result {
        Ok(v3_inputs) => {
            println!("✓ V3 inputs created successfully");
            let old_root_bytes = v3_inputs.old_root.to_bytes_be().1;
            let new_root_bytes = v3_inputs.new_root.to_bytes_be().1;
            println!("  Old root: {:?}[..4]", &old_root_bytes[..4.min(old_root_bytes.len())]);
            println!("  New root: {:?}[..4]", &new_root_bytes[..4.min(new_root_bytes.len())]);
            println!("  Start index: {}", v3_inputs.start_index);
            println!("  Batch size: {}", v3_inputs.batch_size);

            // TODO: V2 approach for comparison once we implement changelog logic
            // let v2_inputs = build_v2_circuit_inputs(...);
            // assert_eq!(v3_inputs, v2_inputs, "V2 and V3 should produce identical circuit inputs");
        }
        Err(e) => {
            eprintln!("✗ V3 failed: {}", e);
            panic!("V3 circuit input generation failed");
        }
    }

    println!("\n=== Test Complete ===\n");
}

/// V3 approach: Build circuit inputs using dedup nodes and StagingTree
fn build_v3_circuit_inputs(
    initial_root: [u8; 32],
    start_index: u32,
    leaves: &[[u8; 32]],
    leaves_hashchain: [u8; 32],
    old_leaves: &[[u8; 32]],
    _merkle_proofs: &[Vec<[u8; 32]>],
    zkp_batch_size: u32,
    leaf_indices: &[u64],
) -> Result<BatchAppendsCircuitInputs, String> {
    // Simulate dedup nodes from Photon (in real test, query from DB)
    let nodes = vec![];
    let node_hashes = vec![];

    // Build staging tree
    let mut staging_tree = StagingTree::from_v2_output_queue(
        leaf_indices,
        old_leaves, // These are the current leaves at those positions
        &nodes,
        &node_hashes,
        initial_root,
    ).map_err(|e| format!("Failed to build staging tree: {}", e))?;

    // Process batch to get old_leaves and proofs
    let (computed_old_leaves, computed_proofs, old_root, new_root) = staging_tree
        .process_batch_updates(leaf_indices, leaves, "TEST", 0)
        .map_err(|e| format!("Failed to process batch: {}", e))?;

    // Build circuit inputs
    get_batch_append_inputs_v2::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
        old_root,
        start_index,
        leaves.to_vec(),
        leaves_hashchain,
        computed_old_leaves,
        computed_proofs,
        zkp_batch_size,
        new_root,
    )
    .map_err(|e| format!("Failed to build circuit inputs: {}", e))
}

// TODO: Implement v2 approach with changelogs for comparison
// fn build_v2_circuit_inputs(...) -> Result<BatchAppendsCircuitInputs, String> {
//     // Use light_sparse_merkle_tree::changelog::ChangelogEntry
//     // Build tree from changelogs
//     // Get old_leaves and proofs
//     // Compare with v3 approach
// }

#[test]
fn test_staging_tree_proof_consistency() {
    println!("\n=== Testing StagingTree Proof Consistency ===\n");

    // Create a simple tree with known values
    let leaf_indices = vec![0u64, 1u64];
    let old_leaves = vec![[0u8; 32], [0u8; 32]];
    let new_leaves = vec![[1u8; 32], [2u8; 32]];

    // Build minimal dedup nodes
    let nodes = vec![];
    let node_hashes = vec![];
    let initial_root = [0u8; 32]; // Empty tree root

    let mut staging_tree = StagingTree::from_v2_output_queue(
        &leaf_indices,
        &old_leaves,
        &nodes,
        &node_hashes,
        initial_root,
    ).expect("Failed to build staging tree");

    println!("Initial root: {:?}[..8]", &staging_tree.current_root()[..8]);

    // Update leaves
    let result = staging_tree.process_batch_updates(
        &leaf_indices,
        &new_leaves,
        "TEST",
        0,
    );

    match result {
        Ok((computed_old_leaves, proofs, old_root, new_root)) => {
            println!("✓ Batch processed successfully");
            println!("  Old root: {:?}[..8]", &old_root[..8]);
            println!("  New root: {:?}[..8]", &new_root[..8]);
            println!("  Old leaves: {} items", computed_old_leaves.len());
            println!("  Proofs: {} items", proofs.len());

            // Verify proof lengths
            for (i, proof) in proofs.iter().enumerate() {
                assert_eq!(
                    proof.len(),
                    DEFAULT_BATCH_STATE_TREE_HEIGHT as usize,
                    "Proof {} should have {} siblings",
                    i,
                    DEFAULT_BATCH_STATE_TREE_HEIGHT
                );
            }

            // Verify we can validate the first proof
            let mut current_hash = computed_old_leaves[0];
            let mut current_index = leaf_indices[0] as usize;
            for sibling in proofs[0].iter() {
                current_hash = if current_index % 2 == 0 {
                    Poseidon::hashv(&[&current_hash[..], &sibling[..]]).unwrap()
                } else {
                    Poseidon::hashv(&[&sibling[..], &current_hash[..]]).unwrap()
                };
                current_index /= 2;
            }

            println!("  Proof validation: computed root {:?}[..8]", &current_hash[..8]);
            assert_eq!(current_hash, old_root, "Proof should validate against old root");

            println!("✓ All checks passed");
        }
        Err(e) => {
            eprintln!("✗ Failed: {}", e);
            panic!("Staging tree update failed");
        }
    }

    println!("\n=== Test Complete ===\n");
}
