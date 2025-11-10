# Insert Into Output Queue

**path:** src/queue.rs

**description:**
Inserts a compressed account hash into the output queue's currently processing batch. Output queues store compressed account hashes until the batch is zeroed (which occurs after the batch is fully inserted into the tree AND the next batch reaches 50% capacity).

Key characteristics:
1. Inserts values into value vec (for immediate spending via proof-by-index)
2. Updates hash chain (for ZKP verification)
3. Automatically transitions batches when full (Fill → Full state when num_inserted_elements reaches batch_size)
4. Assigns leaf index at insertion (increments next_index; tree insertion order is determined at queue insertion)
5. No bloom filters (only input queues use bloom filters)

The output queue uses a two-batch alternating system. The alternating batch system is not strictly necessary for output queues (no bloom filters to zero out), but is used to unify input and output queue code.

Output queues enable **immediate spending**: Values can be spent via proof-by-index before tree insertion. Unlike input queues that only store bloom filters, output queues store actual values in value vecs for proof-by-index. Hash chains are used as public inputs when verifying the ZKP that appends this batch to the tree.

**Operation:**
Method: `BatchedQueueAccount::insert_into_current_batch`

**Parameters:**
- `hash_chain_value`: &[u8; 32] - Compressed account hash to insert
- `current_slot`: &u64 - Current Solana slot number (sets batch start_slot on first insertion; used by indexers to track when batch started filling, not used for batch logic)

**Accounts:**
This operation modifies a `BatchedQueueAccount`:
- Must be type `QueueType::OutputStateV2`
- Account layout defined in: src/queue.rs
- Must have been initialized via `initialize_state_tree`
- Associated with a state Merkle tree

**Operation Logic and Checks:**

1. **Get current insertion index:**
   - Read `batch_metadata.next_index` to determine leaf index for this value
   - This index is used for proof-by-index when spending compressed accounts
   - Assigned at insertion time and determines the leaf position in the tree

2. **Insert into current batch:**
   Calls `insert_into_current_queue_batch` helper which:

   a. **Check batch state (readiness):**
      - If batch state is `Fill`: Ready for insertion, continue
      - If batch state is `Inserted`: Batch was fully processed, needs clearing:
        - Clear value vec (reset all values to zero)
        - Clear hash chain stores (reset all hash chains)
        - Advance batch state to `Fill`
        - Reset batch metadata (start_index, sequence_number, etc.)
      - If batch state is `Full`: Error - batch not ready for insertion

   b. **Insert value into batch:**
      - Call `current_batch.store_and_hash_value`:
        - Store hash_chain_value in value vec at next position
        - Update hash chain:
          - Get current ZKP batch index
          - Hash: `Poseidon(prev_hash_chain, hash_chain_value)`
          - Store updated hash chain in hash chain store
        - Increment batch's internal element counter

   c. **Check if batch is full:**
      - If `num_inserted_elements == batch_size`:
        - Transition batch state from `Fill` to `Full`
        - Increment `currently_processing_batch_index` (switches to other batch)
        - Update `pending_batch_index` (marks this batch ready for tree update)

3. **Increment queue next_index:**
   - `batch_metadata.next_index += 1`
   - The assigned leaf index in the tree (tree insertion order is determined at queue insertion)

**Validations:**
- Batch must be in `Fill` or `Inserted` state (enforced by `insert_into_current_queue_batch`)
- Tree must not be full: `next_index < tree_capacity` (checked by caller before insertion)

**State Changes:**
- Value vec: Stores compressed account hash at index position
- Hash chain store: Updates running Poseidon hash for ZKP batch
- Batch metadata:
  - `num_inserted_elements`: Incremented
  - `state`: May transition `Fill` → `Full` when batch fills
  - `currently_processing_batch_index`: May switch to other batch
  - `pending_batch_index`: Updated when batch becomes full
- Queue metadata:
  - `next_index`: Always incremented (leaf index for this value)

**Errors:**
- `BatchedMerkleTreeError::TreeIsFull` (error code: 14310) - Output queue has reached tree capacity (next_index >= tree_capacity)
- `BatchedMerkleTreeError::BatchNotReady` (error code: 14301) - Batch is in `Full` state and cannot accept insertions
- `BatchedMerkleTreeError::BloomFilterNotZeroed` (error code: 14312) - N/A for output queues (no bloom filters)
- `ZeroCopyError` - Failed to access value vec or hash chain stores
