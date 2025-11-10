# Insert Into Input Queue (Nullifier)

**path:** src/merkle_tree.rs

**description:**
Inserts a nullifier into the state tree's integrated input queue when spending a compressed account. The bloom filter prevents double-spending by checking that the compressed account hash doesn't already exist in any batch's bloom filter. The nullifier (which will replace the compressed account hash in the tree once inserted by a batch update) is stored in the hash chain. The input queue stores nullifiers in hash chains and compressed account hashes in bloom filters until the bloom filter is zeroed (which occurs after the batch is fully inserted into the tree AND the next batch reaches 50% capacity).

Key characteristics:
1. Creates nullifier: `Hash(compressed_account_hash, leaf_index, tx_hash)`
2. Inserts nullifier into hash chain (value that will replace the leaf in the tree)
3. Inserts compressed_account_hash into bloom filter (for non-inclusion checks in subsequent transactions)
4. Checks non-inclusion: compressed_account_hash must not exist in any bloom filter (prevents double-spending)
5. Increments nullifier_next_index (nullifier queue index; used by indexers as sequence number)

The input queue uses a two-batch alternating system to enable zeroing out one bloom filter while the other is still being used for non-inclusion checks.

**Operation:**
Method: `BatchedMerkleTreeAccount::insert_nullifier_into_queue`

**Parameters:**
- `compressed_account_hash`: &[u8; 32] - Hash of compressed account being nullified
- `leaf_index`: u64 - Index in the tree where the compressed account exists (note: although leaf_index is already inside the compressed_account_hash, it's added to the nullifier hash to expose it efficiently in the batch update ZKP)
- `tx_hash`: &[u8; 32] - Transaction hash; enables ZK proofs showing how a compressed account was spent and what other accounts exist in that transaction
- `current_slot`: &u64 - Current Solana slot number (sets batch start_slot on first insertion; used by indexers to track when batch started filling, not used for batch logic)

**Accounts:**
This operation modifies a `BatchedMerkleTreeAccount`:
- Must be type `TreeType::StateV2` (we nullify state not addresses)
- Account layout defined in: src/merkle_tree.rs
- Account documentation: TREE_ACCOUNT.md
- Is initialized via `initialize_state_tree`
- Has integrated input queue (bloom filters + hash chains)

**Operation Logic and Checks:**

1. **Verify tree type:**
   - Check: `tree_type == TreeType::StateV2`
   - Error if address tree

2. **Create nullifier:**
   - Compute: `nullifier = Hash(compressed_account_hash, leaf_index, tx_hash)`
   - Nullifier is transaction-specific (depends on tx_hash)
   - Note, a nullifier could be any value other than the original compressed_account_hash. The only requirement is that post nullifier insertion we cannot prove inclusion of the original compressed_account_hash in the tree.

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

   b. **Insert values into batch:**
      - Call `current_batch.insert`:
        - Insert compressed_account_hash into bloom filter (NOT the nullifier, since nullifier is tx-specific)
        - Check non-inclusion: compressed_account_hash must not exist in any other bloom filter
        - Update hash chain with nullifier: `Poseidon(prev_hash_chain, nullifier)`
        - Store updated hash chain in hash chain store
        - Increment batch's internal element counter

   c. **Check if batch is full:**
      - If `num_inserted_elements == batch_size`:
        - Transition batch state from `Fill` to `Full`
        - Increment `currently_processing_batch_index` (switches to other batch)
        - Update `pending_batch_index` (marks this batch ready for tree update)

4. **Increment nullifier_next_index:**
   - `nullifier_next_index += 1`
   - Used as sequence number by indexers to track nullifier order

**Validations:**
- Tree must be state tree (enforced by tree type check)
- Batch must be in `Fill` or `Inserted` state (enforced by `insert_into_current_queue_batch`)
- Bloom filter must be zeroed before reuse (enforced when clearing batch in `Inserted` state)
- Non-inclusion check: compressed_account_hash must not exist in any bloom filter (prevents double-spending)

**State Changes:**
- Bloom filter: Stores compressed_account_hash for non-inclusion checks
- Hash chain store: Updates running Poseidon hash with nullifier for ZKP batch
- Batch metadata:
  - `num_inserted_elements`: Incremented
  - `state`: May transition `Fill` → `Full` when batch fills
  - `currently_processing_batch_index`: May switch to other batch
  - `pending_batch_index`: Updated when batch becomes full
- Tree metadata:
  - `nullifier_next_index`: Always incremented (sequence number for indexers)

**Errors:**
- `MerkleTreeMetadataError::InvalidTreeType` - Tree is not a state tree (address trees don't support nullifiers)
- `BatchedMerkleTreeError::BatchNotReady` (error code: 14301) - Batch is in `Full` state and cannot accept insertions
- `BatchedMerkleTreeError::BloomFilterNotZeroed` (error code: 14312) - Attempting to reuse batch before bloom filter has been zeroed by forester
- `BatchedMerkleTreeError::NonInclusionCheckFailed` (error code: 14311) - compressed_account_hash already exists in bloom filter (double-spend attempt)
- `ZeroCopyError` - Failed to access bloom filter stores or hash chain stores
