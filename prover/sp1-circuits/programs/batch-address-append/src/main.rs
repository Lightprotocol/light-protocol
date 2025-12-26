//! BatchAddressAppend SP1 Program
//!
//! This program proves the correctness of a batch address append operation on an indexed Merkle tree.
//! It runs in the SP1 zkVM and generates a proof of correct execution.
//!
//! Circuit logic (matching Gnark's BatchAddressTreeAppendCircuit):
//! 1. For each new element:
//!    - Verify range: low_value < new_value < low_next_value
//!    - Update low element to point to new element
//!    - Insert new element at next available position
//! 2. Verify final root equals newRoot
//! 3. Verify new element values hash chain matches hashchainHash
//! 4. Verify public input hash = H(oldRoot, newRoot, hashchainHash, startIndex)

#![no_main]
sp1_zkvm::entrypoint!(main);

use sp1_circuits_lib::{
    hash_chain, index_to_path_bits, leaf_hash_with_range_check, merkle_root_update, poseidon2,
    u32_to_bytes, BatchAddressAppendInputs, Hash,
};

/// Zero value for empty leaf slots (same as Gnark's getZeroValue(0))
const ZERO_VALUE: Hash = [0u8; 32];

pub fn main() {
    // Read inputs from the prover
    let inputs: BatchAddressAppendInputs = sp1_zkvm::io::read();

    // Parse hex strings to bytes
    let parsed = inputs
        .parse()
        .expect("Failed to parse BatchAddressAppend inputs");

    // Verify input dimensions
    assert_eq!(
        parsed.low_element_values.len(),
        parsed.batch_size as usize,
        "Wrong number of low_element_values"
    );
    assert_eq!(
        parsed.low_element_indices.len(),
        parsed.batch_size as usize,
        "Wrong number of low_element_indices"
    );
    assert_eq!(
        parsed.low_element_next_values.len(),
        parsed.batch_size as usize,
        "Wrong number of low_element_next_values"
    );
    assert_eq!(
        parsed.new_element_values.len(),
        parsed.batch_size as usize,
        "Wrong number of new_element_values"
    );
    assert_eq!(
        parsed.low_element_proofs.len(),
        parsed.batch_size as usize,
        "Wrong number of low_element_proofs"
    );
    assert_eq!(
        parsed.new_element_proofs.len(),
        parsed.batch_size as usize,
        "Wrong number of new_element_proofs"
    );

    // 1. Verify public input hash
    // publicInputHash = H(H(H(oldRoot, newRoot), hashchainHash), startIndex)
    let start_index_bytes = u32_to_bytes(parsed.start_index);
    let computed_public_input_hash = hash_chain(&[
        parsed.old_root,
        parsed.new_root,
        parsed.hashchain_hash,
        start_index_bytes,
    ])
    .expect("Failed to compute public input hash");

    assert_eq!(
        computed_public_input_hash, parsed.public_input_hash,
        "Public input hash mismatch"
    );

    // 2. Verify new element values hash chain
    let computed_hashchain = hash_chain(&parsed.new_element_values)
        .expect("Failed to compute new element values hash chain");
    assert_eq!(
        computed_hashchain, parsed.hashchain_hash,
        "New element values hash chain mismatch"
    );

    // 3. Process each element in the batch
    let mut current_root = parsed.old_root;

    for i in 0..parsed.batch_size as usize {
        let low_value = &parsed.low_element_values[i];
        let low_next_value = &parsed.low_element_next_values[i];
        let new_value = &parsed.new_element_values[i];
        let low_index = parsed.low_element_indices[i];

        // Step 3a: Compute old low leaf hash with range check
        // This verifies: low_value < new_value < low_next_value
        // Returns: H(low_value, low_next_value)
        let old_low_leaf_hash = leaf_hash_with_range_check(low_value, low_next_value, new_value)
            .expect("Range check failed for indexed Merkle tree");

        // Step 3b: Compute new low leaf hash
        // The low element now points to the new element: H(low_value, new_value)
        let new_low_leaf_hash =
            poseidon2(low_value, new_value).expect("Failed to compute new low leaf hash");

        // Step 3c: Update root by replacing old low leaf with new low leaf
        let low_path_bits = index_to_path_bits(low_index, parsed.tree_height as usize);
        current_root = merkle_root_update(
            &current_root,
            &old_low_leaf_hash,
            &new_low_leaf_hash,
            &low_path_bits,
            &parsed.low_element_proofs[i],
        )
        .expect("Low element Merkle root update failed");

        // Step 3d: Compute new element leaf hash
        // The new element points to where the low element used to point: H(new_value, low_next_value)
        let new_element_hash =
            poseidon2(new_value, low_next_value).expect("Failed to compute new element hash");

        // Step 3e: Insert new element at start_index + i (replacing zero)
        let new_index = parsed.start_index + i as u32;
        let new_path_bits = index_to_path_bits(new_index, parsed.tree_height as usize);
        current_root = merkle_root_update(
            &current_root,
            &ZERO_VALUE,
            &new_element_hash,
            &new_path_bits,
            &parsed.new_element_proofs[i],
        )
        .expect("New element Merkle root update failed");
    }

    // 4. Verify final root
    assert_eq!(current_root, parsed.new_root, "New root mismatch");

    // Commit the public input hash (this is what the verifier will see)
    sp1_zkvm::io::commit(&parsed.public_input_hash);
}
