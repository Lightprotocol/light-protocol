# Update Tree From Input Queue

**path:** src/merkle_tree.rs

**description:**
Batch updates Merkle tree from input queue with zero-knowledge proof verification. This operation covers two distinct update types:

1. **Batch Nullify** (State Trees): Nullifies existing leaves by overwriting compressed account hashes with nullifiers
2. **Batch Address Append** (Address Trees): Appends new addresses to the tree using indexed Merkle tree insertion

Both operations process one ZKP batch at a time, verifying correctness of: old root + queue values → new root.

**Circuit implementations:**
- Batch nullify: `prover/server/prover/v2/batch_update_circuit.go`
- Batch address append: `prover/server/prover/v2/batch_address_append_circuit.go`

Key characteristics:
1. Verifies ZKP proving correctness of: old root + queue values → new root
2. Updates tree root
3. Increments tree sequence_number (tracks number of tree updates)
4. For address trees: increments tree next_index by zkp_batch_size
5. For state trees: increments nullifier_next_index (offchain indexer tracking only)
6. Marks ZKP batch as inserted in the queue
7. Transitions batch state to Inserted when all ZKP batches complete
8. Zeros out bloom filter when current batch is 50% inserted

**Operations:**

## Batch Nullify (State Trees)

Method: `BatchedMerkleTreeAccount::update_tree_from_input_queue`

**Parameters:**
- `instruction_data`: InstructionDataBatchNullifyInputs - Contains new_root and compressed ZK proof

**Accounts:**
- `BatchedMerkleTreeAccount` (state tree):
  - Must be type `TreeType::StateV2`
  - Contains integrated input queue with nullifiers
  - Account layout defined in: src/merkle_tree.rs
  - Account documentation: TREE_ACCOUNT.md

**Public inputs for ZKP verification:**
- old_root: Current tree root before update
- new_root: New tree root after batch nullify
- leaves_hash_chain: Hash chain from input queue (nullifiers)
- Public input hash: Hash([old_root, new_root, leaves_hash_chain])

**What the ZKP (circuit) proves:**

The batch update circuit proves that nullifiers have been correctly inserted into the Merkle tree:

1. **Verify public input hash:**
   - Computes Hash([old_root, new_root, leaves_hash_chain])
   - Asserts equals circuit.PublicInputHash

2. **Create and verify nullifiers:**
   - For each position i in batch (zkp_batch_size):
     - Computes nullifier[i] = Hash(Leaves[i], PathIndices[i], TxHashes[i])
     - Where Leaves[i] is the compressed_account_hash being nullified
     - PathIndices[i] is the leaf index in the tree
     - TxHashes[i] is the transaction hash
   - Computes hash chain of all nullifiers
   - Asserts equals circuit.LeavesHashchainHash

3. **Perform Merkle updates:**
   - Initialize running root = circuit.OldRoot
   - For each position i (zkp_batch_size positions):
     - Convert PathIndices[i] to binary (tree height bits)
     - Call MerkleRootUpdateGadget:
       - OldRoot: running root
       - OldLeaf: circuit.OldLeaves[i] (can be 0 if not yet appended, or compressed_account_hash)
       - NewLeaf: nullifier[i]
       - PathIndex: PathIndices[i] as bits
       - MerkleProof: circuit.MerkleProofs[i]
       - Height: tree height
     - Update running root with result
   - Assert final running root equals circuit.NewRoot

4. **Public inputs:** Hash([old_root, new_root, leaves_hash_chain])

**Key circuit characteristics:**
- Path index is included in nullifier hash to ensure correct leaf is nullified even when old_leaf is 0
- Since input and output queues are independent, nullifiers can be inserted before values are appended to the tree
- Merkle proof verifies old_leaf value against onchain root, ensuring correct position
- If old_leaf is 0: value not yet appended, but path index in nullifier ensures correct future position
- If old_leaf is non-zero: should equal compressed_account_hash (verified by Merkle proof)

## Batch Address Append (Address Trees)

Method: `BatchedMerkleTreeAccount::update_tree_from_address_queue`

**Parameters:**
- `instruction_data`: InstructionDataAddressAppendInputs - Contains new_root and compressed ZK proof

**Accounts:**
- `BatchedMerkleTreeAccount` (address tree):
  - Must be type `TreeType::AddressV2`
  - Contains integrated input queue with addresses
  - Account layout defined in: src/merkle_tree.rs
  - Account documentation: TREE_ACCOUNT.md

**Public inputs for ZKP verification:**
- old_root: Current tree root before update
- new_root: New tree root after batch address append
- leaves_hash_chain: Hash chain from address queue (addresses)
- start_index: Tree next_index (where batch append begins)
- Public input hash: Hash([old_root, new_root, leaves_hash_chain, start_index])

**What the ZKP (circuit) proves:**

The batch address append circuit proves that addresses have been correctly appended using indexed Merkle tree insertion:

1. **Initialize running root:**
   - Set current root = circuit.OldRoot

2. **For each address i in batch (zkp_batch_size positions):**

   a. **Update low leaf (insert into sorted linked list):**
   - Compute old low leaf hash:
     - Uses LeafHashGadget to verify old low leaf structure
     - Inputs: LowElementValues[i], LowElementNextValues[i], NewElementValues[i]
     - Verifies low_value < new_address < low_next_value (sorted order)
   - Compute new low leaf hash:
     - Hash(LowElementValues[i], NewElementValues[i])
     - Updates low leaf to point to new address instead of old next value
   - Convert LowElementIndices[i] to binary (tree height bits)
   - Call MerkleRootUpdateGadget:
     - OldRoot: current root
     - OldLeaf: old low leaf hash
     - NewLeaf: new low leaf hash (Hash(low_value, new_address))
     - PathIndex: LowElementIndices[i] as bits
     - MerkleProof: circuit.LowElementProofs[i]
     - Height: tree height
   - Update current root with result

   b. **Insert new leaf:**
   - Compute new leaf hash:
     - Hash(NewElementValues[i], LowElementNextValues[i])
     - New address points to what low leaf previously pointed to
   - Compute insertion index: start_index + i
   - Convert insertion index to binary (tree height bits)
   - Call MerkleRootUpdateGadget:
     - OldRoot: current root (after low leaf update)
     - OldLeaf: 0 (position must be empty)
     - NewLeaf: new leaf hash (Hash(new_address, low_next_value))
     - PathIndex: (start_index + i) as bits
     - MerkleProof: circuit.NewElementProofs[i]
     - Height: tree height
   - Update current root with result

3. **Verify final root:**
   - Assert current root equals circuit.NewRoot

4. **Verify leaves hash chain:**
   - Compute hash chain of all NewElementValues
   - Assert equals circuit.HashchainHash

5. **Verify public input hash:**
   - Compute Hash([old_root, new_root, hash_chain, start_index])
   - Assert equals circuit.PublicInputHash

6. **Public inputs:** Hash([old_root, new_root, leaves_hash_chain, start_index])

**Key circuit characteristics:**
- Performs TWO Merkle updates per address (low leaf update + new leaf insertion)
- Maintains sorted order via indexed Merkle tree linked list structure
- Verifies new address fits between low_value and low_next_value (sorted insertion)
- New leaf position must be empty (old_leaf = 0)
- Enables efficient non-inclusion proofs (prove address not in sorted tree)

## Operation Logic and Checks (Both Operations)

1. **Check tree type:**
   - Nullify: Verify tree type is `TreeType::StateV2`
   - Address: Verify tree type is `TreeType::AddressV2`

2. **Check tree capacity (address trees only):**
   - Verify: `tree.next_index + zkp_batch_size <= tree_capacity`
   - Error if tree would exceed capacity after this batch

3. **Get batch information:**
   - Get `pending_batch_index` from queue (batch ready for tree insertion)
   - Get `first_ready_zkp_batch_index` from batch (next ZKP batch to insert)
   - Verify batch has ready ZKP batches: `num_full_zkp_batches > num_inserted_zkp_batches`

4. **Create public inputs hash:**
   - Get `leaves_hash_chain` from hash chain store for this ZKP batch
   - Get `old_root` from tree root history (most recent root)
   - Nullify: Compute `public_input_hash = Hash([old_root, new_root, leaves_hash_chain])`
   - Address: Get `start_index` from tree, compute `public_input_hash = Hash([old_root, new_root, leaves_hash_chain, start_index])`

5. **Verify ZKP and update tree:**
   Calls `verify_update` which:
   - Nullify: Verifies proof with `verify_batch_update(zkp_batch_size, public_input_hash, proof)`
   - Address: Verifies proof with `verify_batch_address_update(zkp_batch_size, public_input_hash, proof)`
   - Increments sequence_number (tree update counter)
   - Appends new_root to root_history (cyclic buffer)
   - Nullify: Increments nullifier_next_index by zkp_batch_size (offchain indexer tracking)
   - Address: Increments tree next_index by zkp_batch_size (new leaves appended)
   - Returns (old_next_index, new_next_index) for event

6. **Mark ZKP batch as inserted:**
   - Call `mark_as_inserted_in_merkle_tree` on batch:
     - Increment `num_inserted_zkp_batches`
     - If all ZKP batches inserted:
       - Set batch `sequence_number = tree_sequence_number + root_history_capacity` (threshold at which root at root_index has been overwritten in cyclic root history)
       - Set batch `root_index` (identifies root that must not exist when bloom filter is zeroed)
       - Transition batch state to `Inserted`
     - Return batch state for next step

7. **Increment pending_batch_index if batch complete:**
   - If batch state is now `Inserted`:
     - Increment `pending_batch_index` (switches to other batch)

8. **Zero out bloom filter if ready:**
   - Same mechanism as described in UPDATE_FROM_OUTPUT_QUEUE.md
   - See that document for detailed explanation of bloom filter and root zeroing

9. **Return batch event:**
   - Contains merkle_tree_pubkey, batch indices, root info, next_index range
   - Nullify: No output_queue_pubkey
   - Address: No output_queue_pubkey

**Validations:**
- Tree type must match operation (StateV2 for nullify, AddressV2 for address)
- Address trees: Tree must not be full after this batch insertion
- Batch must have ready ZKP batches: `num_full_zkp_batches > num_inserted_zkp_batches`
- Batch must not be in `Inserted` state
- ZKP must verify correctly against public inputs

**State Changes:**

**Tree account (Nullify - State Trees):**
- `nullifier_next_index`: Incremented by zkp_batch_size (offchain indexer tracking)
- `sequence_number`: Incremented by 1 (tracks tree updates)
- `root_history`: New root appended (cyclic buffer, may overwrite oldest)
- Input queue bloom filter: May be zeroed if current batch is 50% inserted AND previous batch is fully inserted AND bloom filter not yet zeroed

**Tree account (Address Append - Address Trees):**
- `next_index`: Incremented by zkp_batch_size (new leaves appended)
- `sequence_number`: Incremented by 1 (tracks tree updates)
- `root_history`: New root appended (cyclic buffer, may overwrite oldest)
- Input queue bloom filter: May be zeroed if current batch is 50% inserted AND previous batch is fully inserted AND bloom filter not yet zeroed

**Input queue (Both):**
- Batch `num_inserted_zkp_batches`: Incremented
- Batch `state`: May transition to `Inserted` when all ZKP batches complete
- Batch `sequence_number`: Set to `tree_sequence_number + root_history_capacity` when batch fully inserted (threshold at which root at root_index has been overwritten in cyclic root history)
- Batch `root_index`: Set when batch fully inserted (identifies root that must not exist when bloom filter is zeroed)
- Queue `pending_batch_index`: May increment when batch complete

**Errors:**
- `MerkleTreeMetadataError::InvalidTreeType` (error code: 14007) - Tree type doesn't match operation
- `MerkleTreeMetadataError::InvalidQueueType` (error code: 14004) - Queue type invalid
- `BatchedMerkleTreeError::TreeIsFull` (error code: 14310) - Address tree would exceed capacity after this batch
- `BatchedMerkleTreeError::BatchNotReady` (error code: 14301) - Batch is not in correct state for insertion
- `BatchedMerkleTreeError::InvalidIndex` (error code: 14309) - Root history is empty or index out of bounds
- `BatchedMerkleTreeError::InvalidBatchIndex` (error code: 14308) - Batch index out of range
- `BatchedMerkleTreeError::CannotZeroCompleteRootHistory` (error code: 14313) - Cannot zero out complete or more than complete root history
- `VerifierError::ProofVerificationFailed` (error code: 13006) - ZKP verification failed (proof is invalid)
- `VerifierError::InvalidPublicInputsLength` (error code: 13004) - Public inputs length doesn't match expected
- `ZeroCopyError` (error codes: 15001-15017) - Failed to access root history or hash chain stores
- `HasherError` (error codes: 7001-7012) - Hashing operation failed
