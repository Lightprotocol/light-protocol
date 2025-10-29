# BatchedQueueAccount

**Description:**
Output queue account for state trees that temporarily stores compressed account hashes. Enables immediate spending of newly created compressed accounts via proof-by-index.

**Note:** In the current implementation, `BatchedQueueAccount` is always an output queue (type `OutputStateV2`). Input queues are integrated into the `BatchedMerkleTreeAccount`.

**Discriminator:** b`queueacc`  `[113, 117, 101, 117, 101, 97, 99, 99]`  (8 bytes)

**Path:**
- Struct: `src/queue.rs` - `BatchedQueueAccount`
- Metadata: `src/queue.rs` - `BatchedQueueMetadata`

## Components

### 1. Metadata (`BatchedQueueMetadata`)
- Queue metadata (queue type, associated merkle tree)
- Batch metadata (`QueueBatches`):
  - Batch sizes (`batch_size`, `zkp_batch_size`)
  - `currently_processing_batch_index`: Index of batch accepting new insertions (Fill state)
  - `pending_batch_index`: Index of batch ready for ZKP processing and tree insertion (Full or being incrementally inserted)
  - Two `Batch` structures tracking state and progress
  - **Note:** These indices can differ, enabling parallel insertion while tree updates from the previous batch are being verified
- Tree capacity
- Hashed merkle tree pubkey
- Hashed queue pubkey

### 2. Value Vecs (`[ZeroCopyVecU64<[u8; 32]>; 2]`)
- Two value vectors, one per batch
- Stores the actual compressed account hashes
- Values accessible by leaf index even before tree insertion
- Enables proof-by-index for immediate spending

### 3. Hash Chain Stores (`[ZeroCopyVecU64<[u8; 32]>; 2]`)
- Two hash chain vectors, one per batch
- Each batch has `batch_size / zkp_batch_size` hash chains
- Each hash chain stores Poseidon hash of all values in that ZKP batch
- Used as public inputs for batch append ZKP verification

**Note:** Output queues do NOT have bloom filters (only input queues use bloom filters).

## Serialization

All deserialization is zero-copy.

**In Solana programs:**
```rust
use light_batched_merkle_tree::queue::BatchedQueueAccount;
use light_account_checks::AccountInfoTrait;

// Deserialize output queue
let queue = BatchedQueueAccount::output_from_account_info(account_info)?;
```

**In client code:**
```rust
use light_batched_merkle_tree::queue::BatchedQueueAccount;

// Deserialize output queue
let queue = BatchedQueueAccount::output_from_bytes(&mut account_data)?;
```

## Account Validation

**`output_from_account_info` checks:**
1. Account owned by Light account compression program (`check_owner` using `light-account-checks`)
2. Account discriminator is `queueacc` (`check_discriminator` using `light-account-checks`)
3. Queue type is `OUTPUT_STATE_QUEUE_TYPE_V2`

**`output_from_bytes` checks (client only):**
1. Account discriminator is `queueacc`
2. Queue type is `OUTPUT_STATE_QUEUE_TYPE_V2`

**Error codes:**
- `AccountError::AccountOwnedByWrongProgram` (12012) - Account not owned by compression program
- `AccountError::InvalidAccountSize` (12006) - Account size less than 8 bytes
- `AccountError::InvalidDiscriminator` (12007) - Discriminator mismatch
- `MerkleTreeMetadataError::InvalidQueueType` - Queue type mismatch

## Associated Operations

- [INITIALIZE_STATE_TREE.md](INITIALIZE_STATE_TREE.md) - Create output queue with state tree
- [INSERT_OUTPUT_QUEUE.md](INSERT_OUTPUT_QUEUE.md) - Insert compressed account hashes
- [UPDATE_FROM_OUTPUT_QUEUE.md](UPDATE_FROM_OUTPUT_QUEUE.md) - Update tree from output queue with ZKP

## Supporting Structures

### BatchedQueueMetadata

**Description:**
Metadata for a batched queue account (output queues only).

**Path:** `src/queue.rs`

**Key Fields:**
- `metadata`: Base `QueueMetadata` (queue type, associated merkle tree)
- `batch_metadata`: `QueueBatches` structure
- `tree_capacity`: Associated tree's capacity (2^height). Checked on insertion to prevent overflow
- `hashed_merkle_tree_pubkey`: Pre-hashed tree pubkey (31 bytes + 1 padding). Pubkeys are hashed and truncated to 31 bytes (248 bits) to fit within bn254 field size requirements for Poseidon hashing in ZK circuits
- `hashed_queue_pubkey`: Pre-hashed queue pubkey (31 bytes + 1 padding). Same truncation for bn254 field compatibility
