//! BatchUpdate SP1 Program
//!
//! This program proves the correctness of a batch update (nullification) operation on a Merkle tree.
//! It runs in the SP1 zkVM and generates a proof of correct execution.
//!
//! Circuit logic (matching Gnark's BatchUpdateCircuit):
//! 1. Verify public input hash = H(H(oldRoot, newRoot), leavesHashchainHash)
//! 2. For each leaf:
//!    - Compute nullifier = H(leaf, pathIndex, txHash)
//! 3. Verify nullifier hash chain matches leavesHashchainHash
//! 4. Sequentially update Merkle root, replacing oldLeaf with nullifier
//! 5. Verify final root equals newRoot

#![no_main]
sp1_zkvm::entrypoint!(main);

use sp1_circuits_lib::{
    hash_chain, index_to_path_bits, merkle_root_update, poseidon3, u32_to_bytes,
    BatchUpdateInputs, Hash,
};

pub fn main() {
    // Read inputs from the prover
    let inputs: BatchUpdateInputs = sp1_zkvm::io::read();

    // Parse hex strings to bytes
    let parsed = inputs.parse().expect("Failed to parse BatchUpdate inputs");

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
        parsed.tx_hashes.len(),
        parsed.batch_size as usize,
        "Wrong number of tx_hashes"
    );
    assert_eq!(
        parsed.path_indices.len(),
        parsed.batch_size as usize,
        "Wrong number of path_indices"
    );
    assert_eq!(
        parsed.merkle_proofs.len(),
        parsed.batch_size as usize,
        "Wrong number of merkle_proofs"
    );

    // 1. Verify public input hash
    // publicInputHash = H(H(oldRoot, newRoot), leavesHashchainHash)
    // This is a hash chain of 3 inputs: [oldRoot, newRoot, leavesHashchainHash]
    let computed_public_input_hash = hash_chain(&[
        parsed.old_root,
        parsed.new_root,
        parsed.leaves_hashchain_hash,
    ])
    .expect("Failed to compute public input hash");

    assert_eq!(
        computed_public_input_hash, parsed.public_input_hash,
        "Public input hash mismatch"
    );

    // 2. Compute nullifiers for each leaf
    // nullifier[i] = poseidon3(leaf[i], pathIndex[i], txHash[i])
    let nullifiers: Vec<Hash> = (0..parsed.batch_size as usize)
        .map(|i| {
            let path_index_bytes = u32_to_bytes(parsed.path_indices[i]);
            poseidon3(&parsed.leaves[i], &path_index_bytes, &parsed.tx_hashes[i])
                .expect("Failed to compute nullifier")
        })
        .collect();

    // 3. Verify nullifier hash chain
    let computed_nullifier_hash = hash_chain(&nullifiers).expect("Failed to compute nullifier hash chain");
    assert_eq!(
        computed_nullifier_hash, parsed.leaves_hashchain_hash,
        "Nullifier hash chain mismatch"
    );

    // 4. Sequentially update Merkle root
    let mut current_root = parsed.old_root;

    for i in 0..parsed.batch_size as usize {
        let path_bits = index_to_path_bits(parsed.path_indices[i], parsed.height as usize);

        // Verify old leaf and compute new root with nullifier
        current_root = merkle_root_update(
            &current_root,
            &parsed.old_leaves[i],
            &nullifiers[i],
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
