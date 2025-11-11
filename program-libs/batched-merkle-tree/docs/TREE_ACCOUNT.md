# BatchedMerkleTreeAccount

**Description:**
The main Merkle tree account that stores tree roots, root history, and integrated input queue (bloom filters + hash chains for nullifiers or addresses). Used for both state trees and address trees.

**Discriminator:** b`BatchMta`  `[66, 97, 116, 99, 104, 77, 116, 97]`  (8 bytes)

**Path:**
- Struct: `src/merkle_tree.rs` - `BatchedMerkleTreeAccount`
- Metadata: `src/merkle_tree_metadata.rs` - `BatchedMerkleTreeMetadata`

## Components

### 1. Metadata (`BatchedMerkleTreeMetadata`)
- Tree type: `TreeType::StateV2` or `TreeType::AddressV2`
- Tree height and capacity (2^height leaves)
- Sequence number (increments with each batched tree update (not input or output queue insertions))
- Next index (next available leaf index)
- Nullifier next index (for state trees, address/nullifier queue index)
- Root history capacity
- Queue batch metadata
- Hashed pubkey (31 bytes for bn254 field compatibility)

### 2. Root History (`ZeroCopyCyclicVecU64<[u8; 32]>`)
- Cyclic buffer storing recent tree roots
- Default capacity: 200 roots
- Latest root accessed via `root_history.last()`
- Validity proofs pick root by index from root history
  since proofs need a static root value to verify against.

### 3. Bloom Filter Stores (`[&mut [u8]; 2]`)
- Two bloom filters, one per batch
- Used only for input queues (nullifiers for state trees, addresses for address trees)
- Ensures no duplicate insertions in the queue.
- Zeroed after batch is fully inserted and next batch is 50% full and at least one batch update occured since batch completion.

### 4. Hash Chain Stores (`[ZeroCopyVecU64<[u8; 32]>; 2]`)
- Two hash chain vectors, one per batch (length = `batch_size / zkp_batch_size`)
- Each hash chain stores Poseidon hash of all values in that ZKP batch
- Used as public inputs for ZKP verification

## Tree Type Variants

### State Tree
- Tree type: `STATE_MERKLE_TREE_TYPE_V2`
- Has separate `BatchedQueueAccount` for output operations (appending compressed accounts)
- Uses integrated input queue for nullifier operations
- Initial root: zero bytes root for specified height

### Address Tree
- Tree type: `ADDRESS_MERKLE_TREE_TYPE_V2`
- No separate output queue (only integrated input queue for address insertions)
- Initial root: `ADDRESS_TREE_INIT_ROOT_40` (hardcoded for height 40)
- Starts with next_index = 1 (pre-initialized with one element at index 0)

## Serialization

All deserialization is zero-copy.

**In Solana programs:**
```rust
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_account_checks::AccountInfoTrait;

// Deserialize state tree
let tree = BatchedMerkleTreeAccount::state_from_account_info(account_info)?;

// Deserialize address tree
let tree = BatchedMerkleTreeAccount::address_from_account_info(account_info)?;

// Access root by index
let root = tree.get_root_by_index(index)?;
```

**In client code:**
```rust
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;

// Deserialize state tree
let tree = BatchedMerkleTreeAccount::state_from_bytes(&mut account_data, &pubkey)?;

// Deserialize address tree
let tree = BatchedMerkleTreeAccount::address_from_bytes(&mut account_data, &pubkey)?;
```

## Account Validation

**`state_from_account_info` checks:**
1. Account owned by Light account compression program (`check_owner` using `light-account-checks`)
2. Account discriminator is `BatchMta` (`check_discriminator` using `light-account-checks`)
3. Tree type is `STATE_MERKLE_TREE_TYPE_V2` (4)

**`address_from_account_info` checks:**
1. Account owned by Light account compression program (`check_owner` using `light-account-checks`)
2. Account discriminator is `BatchMta` (`check_discriminator` using `light-account-checks`)
3. Tree type is `ADDRESS_MERKLE_TREE_TYPE_V2` (5)

**`state_from_bytes` checks (client only):**
1. Account discriminator is `BatchMta`
2. Tree type is `STATE_MERKLE_TREE_TYPE_V2` (4)

**`address_from_bytes` checks (client only):**
1. Account discriminator is `BatchMta`
2. Tree type is `ADDRESS_MERKLE_TREE_TYPE_V2` (5)

**Error codes:**
- `AccountError::AccountOwnedByWrongProgram` (12012) - Account not owned by compression program
- `AccountError::InvalidAccountSize` (12006) - Account size less than 8 bytes
- `AccountError::InvalidDiscriminator` (12007) - Discriminator mismatch
- `MerkleTreeMetadataError::InvalidTreeType` - Tree type mismatch (state vs address)

## Associated Operations

- [INITIALIZE_STATE_TREE.md](INITIALIZE_STATE_TREE.md) - Create state tree
- [INITIALIZE_ADDRESS_TREE.md](INITIALIZE_ADDRESS_TREE.md) - Create address tree
- [INSERT_INPUT_QUEUE.md](INSERT_INPUT_QUEUE.md) - Insert nullifiers (state trees)
- [INSERT_ADDRESS_QUEUE.md](INSERT_ADDRESS_QUEUE.md) - Insert addresses (address trees)
- [UPDATE_FROM_INPUT_QUEUE.md](UPDATE_FROM_INPUT_QUEUE.md) - Update tree from input queue with ZKP

## Supporting Structures

### Batch

**Description:**
State machine tracking the lifecycle of a single batch from filling to insertion.

**Path:** `src/batch.rs`

**States:**
- **Fill** (0) - Batch is accepting new insertions. ZKP processing can begin as soon as individual ZKP batches are complete (when `num_full_zkp_batches > 0`)
- **Full** (2) - All ZKP batches are complete (`num_full_zkp_batches == batch_size / zkp_batch_size`). No more insertions accepted
- **Inserted** (1) - All ZKP batches have been inserted into the tree

**State Transitions:**
- Fill → Full: When all ZKP batches are complete (`num_full_zkp_batches == batch_size / zkp_batch_size`)
- Full → Inserted: When all ZKP batches are inserted into tree (`num_inserted_zkp_batches == num_full_zkp_batches`)
- Inserted → Fill: When batch is reset for reuse (after bloom filter zeroing)

**Key Insight:** ZKP processing happens incrementally. A batch doesn't need to be in Full state for ZKP processing to begin - individual ZKP batches can be processed as soon as they're complete, even while the overall batch is still in Fill state.

**Key Fields:**
- `num_inserted`: Number of elements inserted in the current batch
- `num_full_zkp_batches`: Number of ZKP batches ready for insertion
- `num_inserted_zkp_batches`: Number of ZKP batches already inserted into tree
- `sequence_number`: Threshold value set at batch insertion (`tree_seq + root_history_capacity`). Used to detect if sufficient tree updates have occurred since batch insertion to overwrite the last root that was inserted with this batch. When clearing bloom filter, overlapping roots in history must also be zeroed to prevent inclusion proofs of nullified values
- `root_index`: Root index at batch insertion. Identifies which roots in history could prove inclusion of values from this batch's bloom filter. These roots are zeroed when clearing the bloom filter
- `start_index`: Starting leaf index for this batch
- `start_slot`: Slot of first insertion (for indexer reindexing)
- `bloom_filter_is_zeroed`: Whether bloom filter has been zeroed

### QueueBatches

**Description:**
Metadata structure managing the 2-batch system for queues.

**Path:** `src/queue_batch_metadata.rs`

**Key Fields:**
- `num_batches`: Always 2 (alternating batches)
- `batch_size`: Number of elements in a full batch
- `zkp_batch_size`: Number of elements per ZKP batch (batch_size must be divisible by zkp_batch_size)
- `bloom_filter_capacity`: Bloom filter size in bits (0 for output queues)
- `currently_processing_batch_index`: Index of batch accepting new insertions (Fill state)
- `pending_batch_index`: Index of batch ready for ZKP processing and tree insertion (Full or being incrementally inserted)
- `next_index`: Next available leaf index in queue
- `batches`: Array of 2 `Batch` structures

**Variants:**
- **Output Queue** (`new_output_queue`): No bloom filters, has value vecs
- **Input Queue** (`new_input_queue`): Has bloom filters, no value vecs

**Key Validation:**
- `batch_size` must be divisible by `zkp_batch_size`
- Error: `BatchSizeNotDivisibleByZkpBatchSize` if not

### BatchedMerkleTreeMetadata

**Description:**
Complete metadata for a batched Merkle tree account.

**Path:** `src/merkle_tree_metadata.rs`

**Key Fields:**
- `tree_type`: `TreeType::StateV2` (4) or `TreeType::AddressV2` (5)
- `metadata`: Base `MerkleTreeMetadata` (access control, rollover, etc.)
- `sequence_number`: Increments with each tree update
- `next_index`: Next available leaf index in tree
- `nullifier_next_index`: Nullifier sequence tracker (state trees only)
- `height`: Tree height (default: 32 for state, 40 for address)
- `capacity`: Maximum leaves (2^height)
- `root_history_capacity`: Size of root history buffer (default: 200)
- `queue_batches`: Queue batch metadata
- `hashed_pubkey`: Pre-hashed tree pubkey (31 bytes + 1 padding). Pubkeys are hashed and truncated to 31 bytes (248 bits) to fit within bn254 field size requirements for Poseidon hashing in ZK circuits
