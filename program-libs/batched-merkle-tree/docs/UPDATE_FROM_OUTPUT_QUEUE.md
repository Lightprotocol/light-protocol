# Update Tree From Output Queue

**path:** src/merkle_tree.rs

**description:**
Batch appends values from the output queue to the state Merkle tree with zero-knowledge proof verification. This operation processes one ZKP batch at a time, verifying that the tree update from old root + queue values → new root is correct. The ZKP proves that the batch of values from the output queue has been correctly appended to the tree.

**Circuit implementation:** `prover/server/prover/v2/batch_append_circuit.go`

Key characteristics:
1. Verifies ZKP proving correctness of: old root + queue values → new root
2. Updates tree root and increments tree next_index by zkp_batch_size
3. Increments tree sequence_number (tracks number of tree updates)
4. Marks ZKP batch as inserted in the queue
5. Transitions batch state to Inserted when all ZKP batches of a batch are complete
6. Zeros out input queue bloom filter when current batch is 50% inserted

Public inputs for ZKP verification:
- old_root: Current tree root before update
- new_root: New tree root after batch append
- leaves_hash_chain: Hash chain from output queue (commitment to queue values)
- start_index: Tree index where batch append begins

**Operation:**
Method: `BatchedMerkleTreeAccount::update_tree_from_output_queue_account`

**Parameters:**
- `queue_account`: &mut BatchedQueueAccount - Output queue account containing values to append
- `instruction_data`: InstructionDataBatchAppendInputs - Contains new_root and compressed ZK proof

**Accounts:**
This operation modifies:
1. `BatchedMerkleTreeAccount` (state tree):
   - Must be type `TreeType::StateV2`
   - Account layout defined in: src/merkle_tree.rs
   - Account documentation: TREE_ACCOUNT.md

2. `BatchedQueueAccount` (output queue):
   - Must be associated with the state tree (pubkeys match)
   - Account layout defined in: src/queue.rs
   - Account documentation: QUEUE_ACCOUNT.md

**Operation Logic and Checks:**

1. **Check tree is not full:**
   - Verify: `tree.next_index + zkp_batch_size <= tree_capacity`
   - Error if tree would exceed capacity after this batch

2. **Get batch information:**
   - Get `pending_batch_index` from queue (batch ready for tree insertion)
   - Get `first_ready_zkp_batch_index` from batch (next ZKP batch to insert)
   - Verify batch has ready ZKP batches: `num_full_zkp_batches > num_inserted_zkp_batches`
   - Batch can be in `Fill` (being filled) or `Full` state

3. **Create public inputs hash:**
   - Get `leaves_hash_chain` from output queue for this ZKP batch
   - Get `old_root` from tree root history (most recent root)
   - Get `start_index` from tree (where this batch will be appended)
   - Compute: `public_input_hash = Hash([old_root, new_root, leaves_hash_chain, start_index])`

4. **Verify ZKP and update tree:**
   Calls `verify_update` which:
   - Verifies proof: `verify_batch_append_with_proofs(zkp_batch_size, public_input_hash, proof)`
   - Increments tree next_index by zkp_batch_size
   - Increments sequence_number (tree update counter)
   - Appends new_root to root_history (cyclic buffer)
   - Returns (old_next_index, new_next_index) for event

   **What the ZKP (circuit) proves:**
    The batch append circuit proves that a batch of values has been correctly appended to the Merkle tree:
    1. Verifies the public input hash matches Hash([old_root, new_root, leaves_hash_chain, start_index])
    2. Verifies the leaves_hash_chain matches the hash chain of all new leaves
    3. For each position in the batch (zkp_batch_size positions):
      - Checks if old_leaf is zero (empty slot) or non-zero (contains nullifier):
        - If zero: insert the new leaf
        - If non-zero: keep the old leaf (don't overwrite nullified values)
      - Provides Merkle proof for the old leaf value
      - Computes Merkle root update using MerkleRootUpdateGadget
      - Updates running root for next iteration
    4. Verifies the final computed root equals the claimed new_root
    5. Public inputs: Hash([old_root, new_root, leaves_hash_chain, start_index])
    Note: Since input and output queues are independent, a nullifier can be inserted into the tree before the value is appended to the tree. The circuit handles this by checking if the position already contains a nullifier (old_leaf is non-zero) and keeping it instead of overwriting.

5. **Mark ZKP batch as inserted:**
   - Call `mark_as_inserted_in_merkle_tree` on queue batch:
     - Increment `num_inserted_zkp_batches`
     - If all ZKP batches inserted:
       - Set batch `sequence_number = tree_sequence_number + root_history_capacity` (threshold at which root at root_index has been overwritten in cyclic root history)
       - Set batch `root_index` (identifies root that must not exist when bloom filter is zeroed)
       - Transition batch state to `Inserted`
     - Return batch state for next step

6. **Increment pending_batch_index if batch complete:**
   - If batch state is now `Inserted`:
     - Increment `pending_batch_index` (switches to other batch)

7. **Zero out input queue bloom filter if ready:**

   Clears input queue bloom filter after batch insertion to enable batch reuse. This operation runs during both output queue updates AND input queue updates (nullify and address operations).

   **Why zeroing is necessary:**
   - Input queue bloom filters store compressed account hashes to prevent double-spending
   - After batch insertion, old bloom filter values prevent batch reuse (non-inclusion checks fail for legitimate new insertions)
   - Roots from batch insertion period can prove inclusion of bloom filter values
   - Bloom filter must be zeroed to reuse batch; unsafe roots must be zeroed if they still exist in root history

   **When zeroing occurs (all conditions must be true):**
   1. Current batch is at least 50% full: `num_inserted_elements >= batch_size / 2`
   2. Current batch is NOT in `Inserted` state (still being filled)
   3. Previous batch is in `Inserted` state (fully processed)
   4. Previous batch bloom filter NOT already zeroed: `!bloom_filter_is_zeroed()`
   5. At least one tree update occurred since batch completion: `batch.sequence_number != current_tree.sequence_number`

   **Why wait until 50% full:**
   - Zeroing is computationally expensive (foresters perform this, not users)
   - Don't zero when inserting last zkp of batch (would cause failing user transactions)
   - Grace period for clients to switch from proof-by-index to proof-by-zkp for previous batch values

   **Zeroing procedure:**

   a. **Mark bloom filter as zeroed** - Sets flag to prevent re-zeroing

   b. **Zero out bloom filter bytes** - All bytes set to 0

   c. **Zero out overlapping roots** (if any exist):

      **Check for overlapping roots:**
      - Overlapping roots exist if: `batch.sequence_number > current_tree.sequence_number`
      - Cyclic root history has NOT yet overwritten all roots from batch insertion period
      - `batch.sequence_number` was set to `tree_sequence_number + root_history_capacity` at batch completion
      - Represents threshold at which root at `batch.root_index` would be naturally overwritten

      **Calculate unsafe roots:**
      - `num_remaining_roots = batch.sequence_number - current_tree.sequence_number`
      - Roots NOT overwritten since batch insertion
      - These roots can still prove inclusion of bloom filter values
      - `first_safe_root_index = batch.root_index + 1`

      **Safety check:**
      - Verify: `num_remaining_roots < root_history.len()` (never zero complete or more than complete root history)

      **Zero unsafe roots:**
      - Start at `oldest_root_index = root_history.first_index()`
      - Zero `num_remaining_roots` consecutive roots in cyclic buffer
      - Loop wraps: `oldest_root_index = (oldest_root_index + 1) % root_history.len()`
      - Sets each root to `[0u8; 32]`

      **Defensive assertion:**
      - Verify ended at `first_safe_root_index` (ensures correct range zeroed)

   **Why safe:**
   - `sequence_number` mechanism determines when roots are safe to keep
   - Roots at or after `first_safe_root_index` are from updates after batch insertion
   - These roots cannot prove inclusion of zeroed bloom filter values
   - Manual zeroing of overlapping roots prevents cyclic buffer race conditions

8. **Return batch append event:**
   - Contains merkle_tree_pubkey, output_queue_pubkey, batch indices, root info, next_index range

**Validations:**
- Tree must be state tree (enforced by tree type check)
- Tree must not be full after this batch insertion
- Queue and tree must be associated (pubkeys match)
- Batch must have ready ZKP batches: `num_full_zkp_batches > num_inserted_zkp_batches`
- Batch must not be in `Inserted` state
- ZKP must verify correctly against public inputs

**State Changes:**

**Tree account:**
- `next_index`: Incremented by zkp_batch_size (is the leaf index for next insertion)
- `sequence_number`: Incremented by 1 (tracks the number of tree updates)
- `root_history`: New root appended (cyclic buffer, overwrites oldest)
- Input queue bloom filter: May be zeroed if current batch is 50% inserted AND previous batch is fully inserted AND bloom filter not yet zeroed

**Queue account:**
- Batch `num_inserted_zkp_batches`: Incremented
- Batch `state`: May transition to `Inserted` when all ZKP batches complete
- Batch `sequence_number`: Set to `tree_sequence_number + root_history_capacity` when batch fully inserted (threshold at which root at root_index has been overwritten in cyclic root history)
- Batch `root_index`: Set when batch fully inserted (identifies root that must not exist when bloom filter is zeroed)
- Queue `pending_batch_index`: Increments when batch complete

**Errors:**
- `MerkleTreeMetadataError::InvalidTreeType` (error code: 14007) - Tree is not a state tree
- `MerkleTreeMetadataError::MerkleTreeAndQueueNotAssociated` (error code: 14001) - Queue and tree pubkeys don't match
- `BatchedMerkleTreeError::TreeIsFull` (error code: 14310) - Tree would exceed capacity after this batch
- `BatchedMerkleTreeError::BatchNotReady` (error code: 14301) - Batch is not in correct state for insertion
- `BatchedMerkleTreeError::InvalidIndex` (error code: 14309) - Root history is empty or index out of bounds
- `BatchedMerkleTreeError::CannotZeroCompleteRootHistory` (error code: 14313) - Cannot zero out complete or more than complete root history
- `VerifierError::ProofVerificationFailed` (error code: 13006) - ZKP verification failed (proof is invalid)
- `ZeroCopyError` (error codes: 15001-15017) - Failed to access root history or hash chain stores
