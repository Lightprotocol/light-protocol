//! Benchmark for circuit input generation (no prover needed)
//!
//! Run with: cargo test -p forester-utils --release bench_circuit_inputs -- --nocapture

use std::time::Instant;

use light_batched_merkle_tree::constants::DEFAULT_BATCH_STATE_TREE_HEIGHT;
use light_hasher::Poseidon;
use light_merkle_tree_reference::MerkleTree;

use crate::staging_tree::{BatchType, StagingTree};

/// Generate deterministic hash based on index (valid for Poseidon field)
fn hash_from_index(index: u64) -> [u8; 32] {
    // Use a simple hash that stays within field bounds
    // The first byte should be < 0x30 to ensure we're well within the BN254 scalar field
    let mut hash = [0u8; 32];
    let bytes = index.to_le_bytes();
    hash[24..].copy_from_slice(&bytes); // Put index in the last 8 bytes
                                        // Add some variation in middle bytes, keeping values small
    for i in 8..24 {
        hash[i] = ((index as usize * 7 + i * 11) % 200) as u8;
    }
    // First byte small to stay in field
    hash[0] = (index % 32) as u8;
    hash
}

/// Create a synthetic staging tree with pre-populated leaves
fn create_synthetic_staging_tree(num_existing_leaves: usize, height: usize) -> StagingTree {
    // Create leaf indices and hashes for existing leaves
    let leaf_indices: Vec<u64> = (0..num_existing_leaves as u64).collect();
    let leaves: Vec<[u8; 32]> = leaf_indices.iter().map(|&i| hash_from_index(i)).collect();

    // Build a reference tree to get the correct root
    let mut ref_tree = MerkleTree::<Poseidon>::new(height, 0);
    for leaf in &leaves {
        ref_tree.append(leaf).unwrap();
    }
    let initial_root = ref_tree.root();

    // For synthetic test, we don't need intermediate nodes - StagingTree will compute them
    // But we need to provide the leaf data
    StagingTree::new(
        &leaf_indices,
        &leaves,
        &[], // no intermediate nodes needed for fresh tree
        &[],
        initial_root,
        0, // root_seq
        height,
    )
    .expect("Failed to create staging tree")
}

/// Benchmark process_batch_updates for append operations
#[test]
fn bench_circuit_inputs_append() {
    const HEIGHT: usize = DEFAULT_BATCH_STATE_TREE_HEIGHT as usize; // 26
    const BATCH_SIZE: usize = 32; // typical zkp batch size
    const NUM_BATCHES: usize = 10;
    const EXISTING_LEAVES: usize = 1000;

    println!("\n{}", "=".repeat(60));
    println!("CIRCUIT INPUT GENERATION BENCHMARK (Append)");
    println!("{}", "=".repeat(60));
    println!("Tree height: {}", HEIGHT);
    println!("Batch size: {}", BATCH_SIZE);
    println!("Num batches: {}", NUM_BATCHES);
    println!("Existing leaves: {}", EXISTING_LEAVES);
    println!();

    // Create staging tree
    let start = Instant::now();
    let mut staging_tree = create_synthetic_staging_tree(EXISTING_LEAVES, HEIGHT);
    let tree_init_time = start.elapsed();
    println!("Tree initialization: {:?}", tree_init_time);

    let mut batch_times = Vec::new();
    let mut total_leaves_processed = 0;

    for batch_idx in 0..NUM_BATCHES {
        // Generate new leaves for this batch
        let start_index = EXISTING_LEAVES + batch_idx * BATCH_SIZE;
        let leaf_indices: Vec<u64> = (start_index..start_index + BATCH_SIZE)
            .map(|i| i as u64)
            .collect();
        let new_leaves: Vec<[u8; 32]> = leaf_indices
            .iter()
            .map(|&i| hash_from_index(i + 1000000)) // different hash to simulate new data
            .collect();

        let batch_start = Instant::now();

        // This is the expensive operation we're benchmarking
        let result = staging_tree
            .process_batch_updates(
                &leaf_indices,
                &new_leaves,
                BatchType::Append,
                batch_idx,
                batch_idx as u64 + 1,
            )
            .expect("process_batch_updates failed");

        let batch_time = batch_start.elapsed();
        batch_times.push(batch_time);
        total_leaves_processed += BATCH_SIZE;

        println!(
            "Batch {}: {:?} ({} proofs generated)",
            batch_idx,
            batch_time,
            result.merkle_proofs.len()
        );
    }

    // Calculate statistics
    let total_time: std::time::Duration = batch_times.iter().sum();
    let avg_batch_time = total_time / NUM_BATCHES as u32;
    let avg_per_leaf = total_time / total_leaves_processed as u32;

    println!("\n--- Summary ---");
    println!("Total batches: {}", NUM_BATCHES);
    println!("Total leaves: {}", total_leaves_processed);
    println!("Total time: {:?}", total_time);
    println!("Avg per batch: {:?}", avg_batch_time);
    println!("Avg per leaf: {:?}", avg_per_leaf);
    println!(
        "Throughput: {:.1} leaves/sec",
        total_leaves_processed as f64 / total_time.as_secs_f64()
    );
}

/// Benchmark with varying batch sizes
#[test]
fn bench_circuit_inputs_varying_batch_size() {
    const HEIGHT: usize = DEFAULT_BATCH_STATE_TREE_HEIGHT as usize;
    const EXISTING_LEAVES: usize = 1000;
    let batch_sizes = [1, 8, 16, 32, 64, 128, 256];

    println!("\n{}", "=".repeat(60));
    println!("VARYING BATCH SIZE BENCHMARK");
    println!("{}", "=".repeat(60));
    println!("Tree height: {}", HEIGHT);
    println!("Existing leaves: {}", EXISTING_LEAVES);
    println!();

    for &batch_size in &batch_sizes {
        let mut staging_tree = create_synthetic_staging_tree(EXISTING_LEAVES, HEIGHT);

        let leaf_indices: Vec<u64> = (EXISTING_LEAVES..EXISTING_LEAVES + batch_size)
            .map(|i| i as u64)
            .collect();
        let new_leaves: Vec<[u8; 32]> = leaf_indices
            .iter()
            .map(|&i| hash_from_index(i + 1000000))
            .collect();

        let start = Instant::now();
        let result = staging_tree
            .process_batch_updates(&leaf_indices, &new_leaves, BatchType::Append, 0, 1)
            .expect("process_batch_updates failed");
        let elapsed = start.elapsed();

        println!(
            "Batch size {:>3}: {:>10.2?} total, {:>8.2?}/leaf, {:.1} leaves/sec",
            batch_size,
            elapsed,
            elapsed / batch_size as u32,
            batch_size as f64 / elapsed.as_secs_f64()
        );

        assert_eq!(result.merkle_proofs.len(), batch_size);
    }
}

/// Benchmark with varying tree sizes (number of existing leaves)
#[test]
fn bench_circuit_inputs_varying_tree_size() {
    const HEIGHT: usize = DEFAULT_BATCH_STATE_TREE_HEIGHT as usize;
    const BATCH_SIZE: usize = 32;
    let tree_sizes = [100, 1000, 10000, 50000];

    println!("\n{}", "=".repeat(60));
    println!("VARYING TREE SIZE BENCHMARK");
    println!("{}", "=".repeat(60));
    println!("Tree height: {}", HEIGHT);
    println!("Batch size: {}", BATCH_SIZE);
    println!();

    for &num_existing in &tree_sizes {
        let init_start = Instant::now();
        let mut staging_tree = create_synthetic_staging_tree(num_existing, HEIGHT);
        let init_time = init_start.elapsed();

        let leaf_indices: Vec<u64> = (num_existing..num_existing + BATCH_SIZE)
            .map(|i| i as u64)
            .collect();
        let new_leaves: Vec<[u8; 32]> = leaf_indices
            .iter()
            .map(|&i| hash_from_index(i + 1000000))
            .collect();

        let start = Instant::now();
        let _result = staging_tree
            .process_batch_updates(&leaf_indices, &new_leaves, BatchType::Append, 0, 1)
            .expect("process_batch_updates failed");
        let elapsed = start.elapsed();

        println!(
            "Tree size {:>6}: init {:>8.2?}, batch {:>10.2?}, {:>8.2?}/leaf",
            num_existing,
            init_time,
            elapsed,
            elapsed / BATCH_SIZE as u32,
        );
    }
}

/// Profile where time is spent within process_batch_updates
#[test]
fn bench_circuit_inputs_detailed_profiling() {
    const HEIGHT: usize = DEFAULT_BATCH_STATE_TREE_HEIGHT as usize;
    const BATCH_SIZE: usize = 32;
    const EXISTING_LEAVES: usize = 1000;

    println!("\n{}", "=".repeat(60));
    println!("DETAILED PROFILING");
    println!("{}", "=".repeat(60));

    let mut staging_tree = create_synthetic_staging_tree(EXISTING_LEAVES, HEIGHT);

    let leaf_indices: Vec<u64> = (EXISTING_LEAVES..EXISTING_LEAVES + BATCH_SIZE)
        .map(|i| i as u64)
        .collect();
    let new_leaves: Vec<[u8; 32]> = leaf_indices
        .iter()
        .map(|&i| hash_from_index(i + 1000000))
        .collect();

    // Profile individual operations
    let mut get_leaf_times = Vec::new();
    let mut get_proof_times = Vec::new();
    let mut update_times = Vec::new();

    for (&leaf_idx, &new_leaf) in leaf_indices.iter().zip(new_leaves.iter()) {
        // Get old leaf
        let start = Instant::now();
        let old_leaf = staging_tree.get_leaf(leaf_idx);
        get_leaf_times.push(start.elapsed());

        // Get proof
        let start = Instant::now();
        let _proof = staging_tree.get_proof(leaf_idx).unwrap();
        get_proof_times.push(start.elapsed());

        // Determine final leaf
        let is_old_leaf_zero = old_leaf.iter().all(|&byte| byte == 0);
        let final_leaf = if is_old_leaf_zero { new_leaf } else { old_leaf };

        // Update tree
        let start = Instant::now();
        staging_tree.update_leaf(leaf_idx, final_leaf, 1).unwrap();
        update_times.push(start.elapsed());
    }

    let total_get_leaf: std::time::Duration = get_leaf_times.iter().sum();
    let total_get_proof: std::time::Duration = get_proof_times.iter().sum();
    let total_update: std::time::Duration = update_times.iter().sum();
    let total = total_get_leaf + total_get_proof + total_update;

    println!("Batch size: {}", BATCH_SIZE);
    println!();
    println!(
        "get_leaf:   {:>10.2?} ({:>5.1}%)",
        total_get_leaf,
        100.0 * total_get_leaf.as_secs_f64() / total.as_secs_f64()
    );
    println!(
        "get_proof:  {:>10.2?} ({:>5.1}%)",
        total_get_proof,
        100.0 * total_get_proof.as_secs_f64() / total.as_secs_f64()
    );
    println!(
        "update:     {:>10.2?} ({:>5.1}%)",
        total_update,
        100.0 * total_update.as_secs_f64() / total.as_secs_f64()
    );
    println!("─────────────────────────────");
    println!("Total:      {:>10.2?}", total);
    println!();
    println!("Per-leaf averages:");
    println!("  get_leaf:  {:>8.2?}", total_get_leaf / BATCH_SIZE as u32);
    println!("  get_proof: {:>8.2?}", total_get_proof / BATCH_SIZE as u32);
    println!("  update:    {:>8.2?}", total_update / BATCH_SIZE as u32);
}

/// Benchmark the full pipeline including BigInt conversion (BatchAppendsCircuitInputs::new)
#[test]
fn bench_full_append_pipeline() {
    use light_prover_client::proof_types::batch_append::BatchAppendsCircuitInputs;

    const HEIGHT: usize = DEFAULT_BATCH_STATE_TREE_HEIGHT as usize;
    const BATCH_SIZE: usize = 32;
    const EXISTING_LEAVES: usize = 1000;
    const NUM_BATCHES: usize = 10;

    println!("\n{}", "=".repeat(60));
    println!("FULL APPEND PIPELINE BENCHMARK");
    println!("(Including BigInt conversions)");
    println!("{}", "=".repeat(60));
    println!("Tree height: {}", HEIGHT);
    println!("Batch size: {}", BATCH_SIZE);
    println!("Existing leaves: {}", EXISTING_LEAVES);
    println!();

    let mut staging_tree = create_synthetic_staging_tree(EXISTING_LEAVES, HEIGHT);

    let mut process_batch_times = Vec::new();
    let mut circuit_inputs_times = Vec::new();

    for batch_idx in 0..NUM_BATCHES {
        let start_index = EXISTING_LEAVES + batch_idx * BATCH_SIZE;
        let leaf_indices: Vec<u64> = (start_index..start_index + BATCH_SIZE)
            .map(|i| i as u64)
            .collect();
        let new_leaves: Vec<[u8; 32]> = leaf_indices
            .iter()
            .map(|&i| hash_from_index(i + 1000000))
            .collect();

        // Step 1: process_batch_updates (what we benchmarked before)
        let batch_start = Instant::now();
        let result = staging_tree
            .process_batch_updates(
                &leaf_indices,
                &new_leaves,
                BatchType::Append,
                batch_idx,
                batch_idx as u64 + 1,
            )
            .expect("process_batch_updates failed");
        let process_batch_time = batch_start.elapsed();
        process_batch_times.push(process_batch_time);

        // Step 2: BatchAppendsCircuitInputs::new (BigInt conversions + hash chain)
        let leaves_hashchain = hash_from_index(batch_idx as u64 + 2000000);
        let start_idx = leaf_indices.first().copied().unwrap_or(0) as u32;

        let circuit_start = Instant::now();
        let _circuit_inputs =
            BatchAppendsCircuitInputs::new::<{ DEFAULT_BATCH_STATE_TREE_HEIGHT as usize }>(
                result.into(),
                start_idx,
                &new_leaves,
                leaves_hashchain,
                BATCH_SIZE as u32,
            )
            .expect("BatchAppendsCircuitInputs::new failed");
        let circuit_time = circuit_start.elapsed();
        circuit_inputs_times.push(circuit_time);

        println!(
            "Batch {:>2}: process={:>10.2?}, circuit_inputs={:>10.2?}, total={:>10.2?}",
            batch_idx,
            process_batch_time,
            circuit_time,
            process_batch_time + circuit_time
        );
    }

    let total_process: std::time::Duration = process_batch_times.iter().sum();
    let total_circuit: std::time::Duration = circuit_inputs_times.iter().sum();
    let total = total_process + total_circuit;
    let total_leaves = NUM_BATCHES * BATCH_SIZE;

    println!("\n--- Summary ---");
    println!(
        "process_batch_updates: {:>10.2?} ({:>5.1}%)",
        total_process,
        100.0 * total_process.as_secs_f64() / total.as_secs_f64()
    );
    println!(
        "BigInt conversions:    {:>10.2?} ({:>5.1}%)",
        total_circuit,
        100.0 * total_circuit.as_secs_f64() / total.as_secs_f64()
    );
    println!("─────────────────────────────────────────");
    println!("Total:                 {:>10.2?}", total);
    println!();
    println!("Per-leaf: {:>8.2?}", total / total_leaves as u32);
    println!(
        "Throughput: {:.1} leaves/sec",
        total_leaves as f64 / total.as_secs_f64()
    );
}

/// Benchmark tree initialization from indexer-like data
/// This simulates what happens when we fetch data from the indexer
#[test]
fn bench_tree_init_from_indexer_data() {
    const HEIGHT: usize = DEFAULT_BATCH_STATE_TREE_HEIGHT as usize;
    let tree_sizes = [100, 1000, 10000, 50000, 100000];

    println!("\n{}", "=".repeat(60));
    println!("TREE INIT FROM INDEXER DATA BENCHMARK");
    println!("{}", "=".repeat(60));
    println!("Tree height: {}", HEIGHT);
    println!();

    for &num_leaves in &tree_sizes {
        // Simulate indexer data: we need leaf indices, leaves, AND intermediate nodes
        let leaf_indices: Vec<u64> = (0..num_leaves as u64).collect();
        let leaves: Vec<[u8; 32]> = leaf_indices.iter().map(|&i| hash_from_index(i)).collect();

        // Build a reference tree to get intermediate nodes (what indexer would provide)
        let ref_start = Instant::now();
        let mut ref_tree = MerkleTree::<Poseidon>::new(HEIGHT, 0);
        for leaf in &leaves {
            ref_tree.append(leaf).unwrap();
        }
        let initial_root = ref_tree.root();
        let ref_time = ref_start.elapsed();

        // Extract all intermediate nodes from reference tree
        // In production, these come from the indexer
        let mut nodes = Vec::new();
        let mut node_hashes = Vec::new();
        for level in 0..HEIGHT {
            for (pos, hash) in ref_tree.layers[level].iter().enumerate() {
                let node_index = ((level as u64) << 56) | (pos as u64);
                nodes.push(node_index);
                node_hashes.push(*hash);
            }
        }

        // Now benchmark StagingTree::new with all the indexer data
        let staging_start = Instant::now();
        let _staging_tree = StagingTree::new(
            &leaf_indices,
            &leaves,
            &nodes,
            &node_hashes,
            initial_root,
            0,
            HEIGHT,
        )
        .expect("Failed to create staging tree");
        let staging_time = staging_start.elapsed();

        println!(
            "Leaves {:>6}: ref_tree={:>10.2?}, staging_tree={:>10.2?}, nodes={:>6}",
            num_leaves,
            ref_time,
            staging_time,
            nodes.len()
        );
    }
}

/// Create a staging tree WITH intermediate nodes from indexer (more realistic)
fn create_staging_tree_with_nodes(num_existing_leaves: usize, height: usize) -> StagingTree {
    let leaf_indices: Vec<u64> = (0..num_existing_leaves as u64).collect();
    let leaves: Vec<[u8; 32]> = leaf_indices.iter().map(|&i| hash_from_index(i)).collect();

    // Build reference tree to get intermediate nodes
    let mut ref_tree = MerkleTree::<Poseidon>::new(height, 0);
    for leaf in &leaves {
        ref_tree.append(leaf).unwrap();
    }
    let initial_root = ref_tree.root();

    // Extract all intermediate nodes (what indexer provides)
    let mut nodes = Vec::new();
    let mut node_hashes = Vec::new();
    for level in 0..height {
        for (pos, hash) in ref_tree.layers[level].iter().enumerate() {
            let node_index = ((level as u64) << 56) | (pos as u64);
            nodes.push(node_index);
            node_hashes.push(*hash);
        }
    }

    StagingTree::new(
        &leaf_indices,
        &leaves,
        &nodes,
        &node_hashes,
        initial_root,
        0,
        height,
    )
    .expect("Failed to create staging tree")
}

/// Benchmark with indexer-style tree (WITH intermediate nodes)
/// This should match production conditions more closely
#[test]
fn bench_realistic_production_scenario() {
    const HEIGHT: usize = DEFAULT_BATCH_STATE_TREE_HEIGHT as usize;
    const BATCH_SIZE: usize = 32;
    const EXISTING_LEAVES: usize = 8000; // Match production log

    println!("\n{}", "=".repeat(60));
    println!("REALISTIC PRODUCTION SCENARIO");
    println!("(WITH intermediate nodes from indexer)");
    println!("{}", "=".repeat(60));
    println!("Tree height: {}", HEIGHT);
    println!("Batch size: {}", BATCH_SIZE);
    println!("Existing leaves: {}", EXISTING_LEAVES);
    println!();

    // Create tree WITH intermediate nodes (like production)
    let init_start = Instant::now();
    let mut staging_tree = create_staging_tree_with_nodes(EXISTING_LEAVES, HEIGHT);
    let init_time = init_start.elapsed();
    println!("Tree init (with nodes): {:?}", init_time);
    println!();

    for batch_idx in 0..5 {
        let start_index = EXISTING_LEAVES + batch_idx * BATCH_SIZE;
        let leaf_indices: Vec<u64> = (start_index..start_index + BATCH_SIZE)
            .map(|i| i as u64)
            .collect();
        let new_leaves: Vec<[u8; 32]> = leaf_indices
            .iter()
            .map(|&i| hash_from_index(i + 1000000))
            .collect();

        let batch_start = Instant::now();
        let _result = staging_tree
            .process_batch_updates(
                &leaf_indices,
                &new_leaves,
                BatchType::Append,
                batch_idx,
                batch_idx as u64 + 1,
            )
            .expect("process_batch_updates failed");
        let batch_time = batch_start.elapsed();

        println!(
            "Batch {:>2}: {:>10.2?} ({:>8.2?}/leaf)",
            batch_idx,
            batch_time,
            batch_time / BATCH_SIZE as u32
        );
    }
}

/// Create a staging tree with proper intermediate nodes (like production)
fn create_proper_staging_tree(num_existing_leaves: usize, height: usize) -> StagingTree {
    let leaf_indices: Vec<u64> = (0..num_existing_leaves as u64).collect();
    let leaves: Vec<[u8; 32]> = leaf_indices.iter().map(|&i| hash_from_index(i)).collect();

    // Build reference tree to get ALL intermediate nodes
    let mut ref_tree = MerkleTree::<Poseidon>::new(height, 0);
    for leaf in &leaves {
        ref_tree.append(leaf).unwrap();
    }
    let initial_root = ref_tree.root();

    // Extract ALL intermediate nodes (this is what indexer provides)
    let mut nodes = Vec::new();
    let mut node_hashes = Vec::new();
    for level in 0..height {
        for (pos, hash) in ref_tree.layers[level].iter().enumerate() {
            let node_index = ((level as u64) << 56) | (pos as u64);
            nodes.push(node_index);
            node_hashes.push(*hash);
        }
    }

    StagingTree::new(
        &leaf_indices,
        &leaves,
        &nodes,
        &node_hashes,
        initial_root,
        0,
        height,
    )
    .expect("Failed to create staging tree")
}

/// Debug test with small tree
#[test]
fn test_optimized_correctness() {
    const HEIGHT: usize = 4; // Small tree for debugging
    const BATCH_SIZE: usize = 2;
    const EXISTING_LEAVES: usize = 2;

    println!("\n=== CORRECTNESS TEST ===");
    println!(
        "Height: {}, Batch size: {}, Existing: {}",
        HEIGHT, BATCH_SIZE, EXISTING_LEAVES
    );

    // Create two identical trees WITH proper intermediate nodes
    let mut tree_orig = create_proper_staging_tree(EXISTING_LEAVES, HEIGHT);
    let mut tree_opt = create_proper_staging_tree(EXISTING_LEAVES, HEIGHT);

    println!("Initial root (orig): {:?}", &tree_orig.current_root()[..4]);
    println!("Initial root (opt):  {:?}", &tree_opt.current_root()[..4]);

    // Update with same data
    let leaf_indices: Vec<u64> = (EXISTING_LEAVES..EXISTING_LEAVES + BATCH_SIZE)
        .map(|i| i as u64)
        .collect();
    let new_leaves: Vec<[u8; 32]> = leaf_indices
        .iter()
        .map(|&i| hash_from_index(i + 1000000))
        .collect();

    println!("Updating indices: {:?}", leaf_indices);

    let result_orig = tree_orig
        .process_batch_updates(&leaf_indices, &new_leaves, BatchType::Append, 0, 1)
        .expect("original failed");

    let result_opt = tree_opt
        .process_batch_updates_optimized(&leaf_indices, &new_leaves, BatchType::Append, 0, 1)
        .expect("optimized failed");

    println!("New root (orig): {:?}", &result_orig.new_root[..8]);
    println!("New root (opt):  {:?}", &result_opt.new_root[..8]);
    println!("Old root (orig): {:?}", &result_orig.old_root[..8]);
    println!("Old root (opt):  {:?}", &result_opt.old_root[..8]);

    assert_eq!(
        result_orig.old_root, result_opt.old_root,
        "Old roots should match"
    );
    assert_eq!(
        result_orig.new_root, result_opt.new_root,
        "New roots should match"
    );
    println!("PASSED: Roots match!");
}

/// Compare original vs optimized batch update
#[test]
fn bench_optimized_vs_original() {
    const HEIGHT: usize = DEFAULT_BATCH_STATE_TREE_HEIGHT as usize;
    const NUM_BATCHES: usize = 5;
    const EXISTING_LEAVES: usize = 1000;
    let batch_sizes = [8, 16, 32, 64, 128];

    println!("\n{}", "=".repeat(70));
    println!("OPTIMIZED VS ORIGINAL BATCH UPDATE COMPARISON");
    println!("{}", "=".repeat(70));
    println!("Tree height: {}", HEIGHT);
    println!("Existing leaves: {}", EXISTING_LEAVES);
    println!();

    for &batch_size in &batch_sizes {
        // Test original with proper intermediate nodes
        let mut original_times = Vec::new();
        let mut staging_tree_orig = create_staging_tree_with_nodes(EXISTING_LEAVES, HEIGHT);

        for batch_idx in 0..NUM_BATCHES {
            let start_index = EXISTING_LEAVES + batch_idx * batch_size;
            let leaf_indices: Vec<u64> = (start_index..start_index + batch_size)
                .map(|i| i as u64)
                .collect();
            let new_leaves: Vec<[u8; 32]> = leaf_indices
                .iter()
                .map(|&i| hash_from_index(i + 1000000))
                .collect();

            let start = Instant::now();
            let _result = staging_tree_orig
                .process_batch_updates(
                    &leaf_indices,
                    &new_leaves,
                    BatchType::Append,
                    batch_idx,
                    batch_idx as u64 + 1,
                )
                .expect("process_batch_updates failed");
            original_times.push(start.elapsed());
        }

        // Test optimized with proper intermediate nodes
        let mut optimized_times = Vec::new();
        let mut staging_tree_opt = create_staging_tree_with_nodes(EXISTING_LEAVES, HEIGHT);

        for batch_idx in 0..NUM_BATCHES {
            let start_index = EXISTING_LEAVES + batch_idx * batch_size;
            let leaf_indices: Vec<u64> = (start_index..start_index + batch_size)
                .map(|i| i as u64)
                .collect();
            let new_leaves: Vec<[u8; 32]> = leaf_indices
                .iter()
                .map(|&i| hash_from_index(i + 1000000))
                .collect();

            let start = Instant::now();
            let _result = staging_tree_opt
                .process_batch_updates_optimized(
                    &leaf_indices,
                    &new_leaves,
                    BatchType::Append,
                    batch_idx,
                    batch_idx as u64 + 1,
                )
                .expect("process_batch_updates_optimized failed");
            optimized_times.push(start.elapsed());
        }

        // Verify both produce same root
        assert_eq!(
            staging_tree_orig.current_root(),
            staging_tree_opt.current_root(),
            "Roots should match for batch_size={}",
            batch_size
        );

        let original_total: std::time::Duration = original_times.iter().sum();
        let optimized_total: std::time::Duration = optimized_times.iter().sum();
        let speedup = original_total.as_secs_f64() / optimized_total.as_secs_f64();

        println!(
            "Batch size {:>3}: original={:>10.2?}, optimized={:>10.2?}, speedup={:.2}x",
            batch_size,
            original_total / NUM_BATCHES as u32,
            optimized_total / NUM_BATCHES as u32,
            speedup
        );
    }
}
