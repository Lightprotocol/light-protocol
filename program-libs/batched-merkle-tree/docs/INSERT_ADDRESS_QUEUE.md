# Insert Into Address Queue

**path:** src/merkle_tree.rs

**description:**
Inserts an address into the address tree's integrated address queue when creating a new address for compressed accounts. The bloom filter prevents address reuse by checking that the address doesn't already exist in any batch's bloom filter. The address is stored in the hash chain and will be inserted into the tree by a batch update. The address queue stores addresses in both hash chains and bloom filters until the bloom filter is zeroed (which occurs after the batch is fully inserted into the tree AND the next batch reaches 50% capacity AND at least one batch update has occurred since batch completion).

Key characteristics:
1. Inserts address into both bloom filter and hash chain (same value in both)
2. Checks non-inclusion: address must not exist in any bloom filter (prevents address reuse)
3. Checks tree capacity before insertion (address trees have fixed capacity)
4. Increments queue_next_index (address queue index; used by indexers as sequence number)

The address queue uses a two-batch alternating system to enable zeroing out one bloom filter while the other is still being used for non-inclusion checks.

**Operation:**
Method: `BatchedMerkleTreeAccount::insert_address_into_queue`

**Parameters:**
- `address`: &[u8; 32] - Address to insert (32-byte hash)
- `current_slot`: &u64 - Current Solana slot number (sets batch start_slot on first insertion; used by indexers to track when batch started filling, not used for batch logic)

**Accounts:**
This operation modifies a `BatchedMerkleTreeAccount`:
- Must be type `TreeType::AddressV2`
- Account layout defined in: src/merkle_tree.rs
- Account documentation: TREE_ACCOUNT.md
- Is initialized via `initialize_address_tree`
- Has integrated address queue (bloom filters + hash chains)

**Operation Logic and Checks:**

1. **Verify tree type:**
   - Check: `tree_type == TreeType::AddressV2`
   - Error if state tree (state trees don't have address queues)

2. **Check tree capacity:**
   - Call `check_queue_next_index_reached_tree_capacity()`
   - Error if `queue_next_index >= tree_capacity`
   - Ensures all queued addresses can be inserted into the tree

3. **Insert into current batch:**
   Calls `insert_into_current_queue_batch` helper which:

   a. **Check batch state (readiness):**
      - If batch state is `Fill`: Ready for insertion, continue
      - If batch state is `Inserted`: Batch was fully processed, needs clearing:
        - Check bloom filter is zeroed; error if not
        - Clear hash chain stores (reset all hash chains)
        - Advance batch state to `Fill`
        - Reset batch metadata (start_index, sequence_number, etc.)
      - If batch state is `Full`: Error - batch not ready for insertion

   b. **Insert address into batch:**
      - Call `current_batch.insert`:
        - Insert address into bloom filter
        - Check non-inclusion: address must not exist in any other bloom filter
        - Update hash chain with address: `Poseidon(prev_hash_chain, address)`
        - Store updated hash chain in hash chain store
        - Increment batch's internal element counter

   c. **Check if batch is full:**
      - If `num_inserted_elements == batch_size`:
        - Transition batch state from `Fill` to `Full`
        - Increment `currently_processing_batch_index` (switches to other batch)
        - Update `pending_batch_index` (marks this batch ready for tree update)

4. **Increment queue_next_index:**
   - `queue_next_index += 1`
   - Used as sequence number by indexers to track address order

**Validations:**
- Tree must be address tree (enforced by tree type check)
- Tree must not be full: `queue_next_index < tree_capacity` (checked before insertion)
- Batch must be in `Fill` or `Inserted` state (enforced by `insert_into_current_queue_batch`)
- Bloom filter must be zeroed before reuse (enforced when clearing batch in `Inserted` state)
- Non-inclusion check: address must not exist in any bloom filter (prevents address reuse)

**State Changes:**
- Bloom filter: Stores address for non-inclusion checks
- Hash chain store: Updates running Poseidon hash with address for ZKP batch
- Batch metadata:
  - `num_inserted_elements`: Incremented
  - `state`: May transition `Fill` → `Full` when batch fills
  - `currently_processing_batch_index`: May switch to other batch
  - `pending_batch_index`: Updated when batch becomes full
- Tree metadata:
  - `queue_next_index`: Always incremented (sequence number for indexers)

**Errors:**
- `MerkleTreeMetadataError::InvalidTreeType` - Tree is not an address tree (state trees don't support address insertion)
- `BatchedMerkleTreeError::TreeIsFull` (error code: 14310) - Address tree has reached capacity (queue_next_index >= tree_capacity)
- `BatchedMerkleTreeError::BatchNotReady` (error code: 14301) - Batch is in `Full` state and cannot accept insertions
- `BatchedMerkleTreeError::BloomFilterNotZeroed` (error code: 14312) - Attempting to reuse batch before bloom filter has been zeroed by forester
- `BatchedMerkleTreeError::NonInclusionCheckFailed` (error code: 14311) - Address already exists in bloom filter (address reuse attempt)
- `ZeroCopyError` - Failed to access bloom filter stores or hash chain stores
