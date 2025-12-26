//! BatchAppend SP1 Program
//!
//! This program proves the correctness of a batch append operation on a Merkle tree.
//! It runs in the SP1 zkVM and generates a proof of correct execution.
//!
//! Circuit logic (matching Gnark's BatchAppendCircuit):
//! 1. Verify public input hash = H(oldRoot, newRoot, leavesHashchainHash, startIndex)
//! 2. For each leaf position:
//!    - If oldLeaf == 0: newLeaf = leaf (append new)
//!    - Else: newLeaf = oldLeaf (keep existing)
//! 3. Verify leavesHashchainHash = H(H(H(leaf[0], leaf[1]), leaf[2]), ...)
//! 4. Sequentially update Merkle root for each leaf
//! 5. Verify final root equals newRoot

#![no_main]
sp1_zkvm::entrypoint!(main);

use sp1_circuits_lib::{
    hash_chain, index_to_path_bits, merkle_root_update, u32_to_bytes,
    BatchAppendInputs, Hash,
};

pub fn main() {
    // Read inputs from the prover
    let inputs: BatchAppendInputs = sp1_zkvm::io::read();

    // Parse hex strings to bytes
    let parsed = inputs
        .parse()
        .expect("Failed to parse BatchAppend inputs");

    // Verify input dimensions
    assert_eq!(
        parsed.leaves.len(),
        parsed.batch_size as usize,
        "Wrong number of leaves"
    );
    assert_eq!(
        parsed.old_leaves.len(),
        parsed.batch_size as usize,
        "Wrong number of old_leaves"
    );
    assert_eq!(
        parsed.merkle_proofs.len(),
        parsed.batch_size as usize,
        "Wrong number of merkle_proofs"
    );

    // 1. Verify public input hash
    // publicInputHash = H(H(H(oldRoot, newRoot), leavesHashchainHash), startIndex)
    // This is computed as a hash chain: H(H(H(a,b),c),d)
    let start_index_bytes = u32_to_bytes(parsed.start_index);
    let computed_public_input_hash = hash_chain(&[
        parsed.old_root,
        parsed.new_root,
        parsed.leaves_hashchain_hash,
        start_index_bytes,
    ])
    .expect("Failed to compute public input hash");

    assert_eq!(
        computed_public_input_hash, parsed.public_input_hash,
        "Public input hash mismatch"
    );

    // 2. Compute new leaves based on old leaves
    // If oldLeaf == 0, use new leaf; otherwise keep old leaf
    let new_leaves: Vec<Hash> = (0..parsed.batch_size as usize)
        .map(|i| {
            if is_zero(&parsed.old_leaves[i]) {
                parsed.leaves[i]
            } else {
                parsed.old_leaves[i]
            }
        })
        .collect();

    // 3. Verify leaves hash chain
    let computed_leaves_hash = hash_chain(&parsed.leaves).expect("Failed to compute leaves hash chain");
    assert_eq!(
        computed_leaves_hash, parsed.leaves_hashchain_hash,
        "Leaves hash chain mismatch"
    );

    // 4. Sequentially update Merkle root
    let mut current_root = parsed.old_root;

    for i in 0..parsed.batch_size as usize {
        let index = parsed.start_index + i as u32;
        let path_bits = index_to_path_bits(index, parsed.height as usize);

        // Verify old leaf and compute new root
        current_root = merkle_root_update(
            &current_root,
            &parsed.old_leaves[i],
            &new_leaves[i],
            &path_bits,
            &parsed.merkle_proofs[i],
        )
        .expect("Merkle root update failed");
    }

    // 5. Verify final root
    assert_eq!(current_root, parsed.new_root, "New root mismatch");

    // Commit the public input hash (this is what the verifier will see)
    sp1_zkvm::io::commit(&parsed.public_input_hash);
}

/// Check if a hash is all zeros.
fn is_zero(hash: &Hash) -> bool {
    hash.iter().all(|&b| b == 0)
}
