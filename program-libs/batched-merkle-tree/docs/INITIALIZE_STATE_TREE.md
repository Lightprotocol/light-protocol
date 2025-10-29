# Initialize State Tree

**path:** src/initialize_state_tree.rs

**description:**
Initializes a state tree with integrated input queue and separate output queue. This operation creates **two Solana accounts**:

1. **State Merkle tree account** (`BatchedMerkleTreeAccount`) - Stores tree roots, root history, and integrated input queue (bloom filters + hash chains for nullifiers)
   - Account layout `BatchedMerkleTreeAccount` defined in: src/merkle_tree.rs
   - Metadata `BatchedMerkleTreeMetadata` defined in: src/merkle_tree_metadata.rs
   - Tree type: `TreeType::StateV2` (4)
   - Initial root: zero bytes for specified height
   - Discriminator: b`BatchMta` `[66, 97, 116, 99, 104, 77, 116, 97]` (8 bytes)

2. **Output queue account** (`BatchedQueueAccount`) - Temporarily stores compressed account hashes before tree insertion
   - Account layout `BatchedQueueAccount` defined in: src/queue.rs
   - Metadata `BatchedQueueMetadata` defined in: src/queue.rs
   - Queue type: `QueueType::OutputStateV2`
   - Enables immediate spending via proof-by-index
   - Discriminator: b`queueacc` `[113, 117, 101, 117, 101, 97, 99, 99]` (8 bytes)

State trees are used for compressed account lifecycle management. The output queue stores newly created compressed accounts, while the input queue (integrated into the tree account) tracks nullifiers when compressed accounts are spent.

**Instruction data:**
Instruction data is defined in: src/initialize_state_tree.rs

`InitStateTreeAccountsInstructionData` struct:

**Tree configuration:**
- `height`: u32 - Tree height (default: 32, capacity = 2^32 leaves)
- `index`: u64 - Unchecked identifier of the state tree
- `root_history_capacity`: u32 - Size of root history cyclic buffer (default: 200)

**Batch sizes:**
- `input_queue_batch_size`: u64 - Elements per input queue batch (default: 15,000)
- `output_queue_batch_size`: u64 - Elements per output queue batch (default: 15,000)
- `input_queue_zkp_batch_size`: u64 - Elements per ZKP batch for nullifications (default: 500)
- `output_queue_zkp_batch_size`: u64 - Elements per ZKP batch for appends (default: 500)

**Validation:** Batch sizes must be divisible by their respective ZKP batch sizes. Error: `BatchSizeNotDivisibleByZkpBatchSize` if validation fails.

**Bloom filter configuration (input queue only):**
- `bloom_filter_capacity`: u64 - Capacity in bits (default: batch_size * 8)
- `bloom_filter_num_iters`: u64 - Number of hash functions (default: 3 for test, 10 for production)

**Validation:**
- Capacity must be divisible by 8
- Capacity must be >= batch_size * 8

**Access control:**
- `program_owner`: Option<Pubkey> - Optional program owning the tree
- `forester`: Option<Pubkey> - Optional forester pubkey for non-Light foresters
- `owner`: Pubkey - Account owner (passed separately as function parameter, not in struct)

**Rollover and fees:**
- `rollover_threshold`: Option<u64> - Percentage threshold for rollover (default: 95%)
- `network_fee`: Option<u64> - Network fee amount (default: 5,000 lamports)
- `additional_bytes`: u64 - CPI context account size for rollover (default: 20KB + 8 bytes)
- `close_threshold`: Option<u64> - Placeholder, unimplemented

**Accounts:**
1. merkle_tree_account
   - mutable
   - State Merkle tree account being initialized
   - Must be rent-exempt for calculated size

2. queue_account
   - mutable
   - Output queue account being initialized
   - Must be rent-exempt for calculated size

Note: No signer accounts required - accounts are expected to be pre-created with correct sizes

**Instruction Logic and Checks:**

1. **Calculate account sizes:**
   - Queue account size: Based on output_queue_batch_size and output_queue_zkp_batch_size
   - Merkle tree account size: Based on input_queue_batch_size, bloom_filter_capacity, input_queue_zkp_batch_size, root_history_capacity, and height
   - Account size formulas defined in: src/queue.rs (`get_output_queue_account_size`) and src/merkle_tree.rs (`get_merkle_tree_account_size`)

2. **Verify rent exemption:**
   - Check: queue_account balance >= minimum rent exemption for queue_account_size
   - Check: merkle_tree_account balance >= minimum rent exemption for mt_account_size
   - Uses: `check_account_balance_is_rent_exempt` from `light-account-checks`
   - Store rent amounts for rollover fee calculation

3. **Initialize output queue account:**
   - Set discriminator: `queueacc` (8 bytes)
   - Create queue metadata:
     - queue_type: `QueueType::OutputStateV2`
     - associated_merkle_tree: merkle_tree_account pubkey
     - Calculate rollover_fee: Based on rollover_threshold, height, and total rent (merkle_tree_rent + additional_bytes_rent + queue_rent)
     - access_metadata: Set owner, program_owner, forester
     - rollover_metadata: Set index, rollover_fee, rollover_threshold, network_fee, close_threshold, additional_bytes
   - Initialize batch metadata:
     - 2 batches (alternating)
     - batch_size: output_queue_batch_size
     - zkp_batch_size: output_queue_zkp_batch_size
     - bloom_filter_capacity: 0 (output queues don't use bloom filters)
   - Initialize value vecs: 2 vectors (one per batch), capacity = batch_size each
   - Initialize hash chain stores: 2 vectors (one per batch), capacity = (batch_size / zkp_batch_size) each
   - Compute hashed pubkeys: Hash and truncate to 31 bytes for bn254 field compatibility
   - tree_capacity: 2^height
   - Rollover fee: Charged when creating output compressed accounts (insertion into output queue)

4. **Initialize state Merkle tree account:**
   - Set discriminator: `BatchMta` (8 bytes)
   - Create tree metadata:
     - tree_type: `TreeType::StateV2` (4)
     - associated_queue: queue_account pubkey
     - access_metadata: Set owner, program_owner, forester
     - rollover_metadata: Set index, rollover_fee=0 (charged on queue insertion, not tree ops), rollover_threshold, network_fee, close_threshold, additional_bytes=None
   - Initialize root history: Cyclic buffer with capacity=root_history_capacity, first entry = zero bytes for tree height
   - Initialize integrated input queue:
     - 2 bloom filter stores (one per batch), size = bloom_filter_capacity / 8 bytes each
     - 2 hash chain stores (one per batch), capacity = (input_queue_batch_size / input_queue_zkp_batch_size) each
     - Batch metadata with input_queue_batch_size and input_queue_zkp_batch_size
   - Compute hashed_pubkey: Hash and truncate to 31 bytes for bn254 field compatibility
   - next_index: 0 (starts empty)
   - sequence_number: 0 (increments with each tree update)

5. **Validate configurations:**
   - root_history_capacity >= (output_queue_batch_size / output_queue_zkp_batch_size) + (input_queue_batch_size / input_queue_zkp_batch_size)
   - Rationale: Ensures sufficient space for roots generated by both input and output operations
   - ZKP batch sizes must be 10 or 500 (only supported circuit sizes)

**Errors:**
- `AccountError::AccountNotRentExempt` (error code: 12011) - Account balance insufficient for rent exemption at calculated size
- `AccountError::InvalidAccountSize` (error code: 12006) - Account data length doesn't match calculated size requirements
- `BatchedMerkleTreeError::BatchSizeNotDivisibleByZkpBatchSize` (error code: 14305) - Batch size is not evenly divisible by ZKP batch size
- `MerkleTreeMetadataError::InvalidRolloverThreshold` - Rollover threshold value is invalid (must be percentage)
- `ZeroCopyError::Size` - Account size mismatch during zero-copy deserialization
- `BorshError` - Failed to serialize or deserialize metadata structures
