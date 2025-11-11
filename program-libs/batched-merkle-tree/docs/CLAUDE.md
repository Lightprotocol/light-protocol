# Batched Merkle Tree Library

The `light-batched-merkle-tree` crate provides batched Merkle tree implementations for the Light Protocol account compression program. Instead of updating trees one leaf at a time, this library batches multiple insertions and updates them with zero-knowledge proofs (ZKPs), enabling efficient on-chain verification. Trees maintain a cyclic root history for validity proofs, and use bloom filters for non-inclusion proofs while batches are being filled.

There are two tree types: **state trees** (two accounts tree account (input queue, tree metadata, roots), output queue account) for compressed accounts, and **address trees** (one account that contains the address queue, tree metadata, roots) for address registration.

## Accounts

### Account Types

- **[TREE_ACCOUNT.md](TREE_ACCOUNT.md)** - BatchedMerkleTreeAccount (state and address trees)
- **[QUEUE_ACCOUNT.md](QUEUE_ACCOUNT.md)** - BatchedQueueAccount (output queue for state trees)

### Overview

The batched merkle tree library uses two main Solana account types:

**BatchedMerkleTreeAccount:**
The main tree account storing tree roots, root history, and integrated input queue (bloom filters + hash chains for nullifiers or addresses). Used for both state trees and address trees.

**Details:** [TREE_ACCOUNT.md](TREE_ACCOUNT.md)

**BatchedQueueAccount:**
Output queue account for state trees that temporarily stores compressed account hashes before tree insertion. Enables immediate spending via proof-by-index.

**Details:** [QUEUE_ACCOUNT.md](QUEUE_ACCOUNT.md)

### State Trees vs Address Trees

**State Trees (2 accounts):**
- `BatchedMerkleTreeAccount` with integrated input queue (for nullifiers)
- Separate `BatchedQueueAccount` for output operations (appending new compressed accounts)

**Address Trees (1 account):**
- `BatchedMerkleTreeAccount` with integrated input queue (for addresses)
- No separate output queue

## Operations

### Initialization
- **[INITIALIZE_STATE_TREE.md](INITIALIZE_STATE_TREE.md)** - Create state tree + output queue pair (2 solana accounts)
  - Source: [`src/initialize_state_tree.rs`](../src/initialize_state_tree.rs)

- **[INITIALIZE_ADDRESS_TREE.md](INITIALIZE_ADDRESS_TREE.md)** - Create address tree with integrated queue (1 solana account)
  - Source: [`src/initialize_address_tree.rs`](../src/initialize_address_tree.rs)

### Queue Insertion Operations
- **[INSERT_OUTPUT_QUEUE.md](INSERT_OUTPUT_QUEUE.md)** - Insert compressed account hash into output queue (state tree)
  - Source: [`src/queue.rs`](../src/queue.rs) - `BatchedQueueAccount::insert_into_current_batch`

- **[INSERT_INPUT_QUEUE.md](INSERT_INPUT_QUEUE.md)** - Insert nullifiers into input queue (state tree)
  - Source: [`src/merkle_tree.rs`](../src/merkle_tree.rs) - `BatchedMerkleTreeAccount::insert_nullifier_into_queue`

- **[INSERT_ADDRESS_QUEUE.md](INSERT_ADDRESS_QUEUE.md)** - Insert addresses into address queue
  - Source: [`src/merkle_tree.rs`](../src/merkle_tree.rs) - `BatchedMerkleTreeAccount::insert_address_into_queue`

### Tree Update Operations
- **[UPDATE_FROM_OUTPUT_QUEUE.md](UPDATE_FROM_OUTPUT_QUEUE.md)** - Batch append with ZKP verification
  - Source: [`src/merkle_tree.rs`](../src/merkle_tree.rs) - `BatchedMerkleTreeAccount::update_tree_from_output_queue_account`

- **[UPDATE_FROM_INPUT_QUEUE.md](UPDATE_FROM_INPUT_QUEUE.md)** - Batch nullify/address updates with ZKP
  - Source: [`src/merkle_tree.rs`](../src/merkle_tree.rs) - `update_tree_from_input_queue`, `update_tree_from_address_queue`

## Key Concepts

**Batching System:** Trees use 2 alternating batches. While one batch is being filled, the previous batch can be updated into the tree with a ZKP.

**ZKP Batches:** Each batch is divided into smaller ZKP batches (`batch_size / zkp_batch_size`). Trees are updated incrementally by ZKP batch.

**Bloom Filters:** Input queues (nullifier queue for state trees, address queue for address trees) use bloom filters for non-inclusion proofs. While a batch is filling, values are inserted into the bloom filter. After the batch is fully inserted into the tree and the next batch is 50% full, the bloom filter is zeroed to prevent false positives. Output queues do not use bloom filters.

**Value Vecs:** Output queues store the actual compressed account hashes in value vectors (one per batch). Values can be accessed by leaf index even before they're inserted into the tree, enabling immediate spending of newly created compressed accounts.

**Hash Chains:** Each ZKP batch has a hash chain storing the Poseidon hash of all values in that ZKP batch. These hash chains are used as public inputs for ZKP verification.

**ZKP Verification:** Tree updates require zero-knowledge proofs proving the correctness of batch operations (old root + queue values → new root). Public inputs: old root, new root, hash chain (commitment to queue elements), and for appends: start_index (output queue) or next_index (address queue).

**Root History:** Trees maintain a cyclic buffer of recent roots (default: 200). This enables validity proofs for recently spent compressed accounts even as the tree continues to update.

**Rollover:** When a tree reaches capacity (2^height leaves), it must be replaced with a new tree. The rollover process creates a new tree and marks the old tree as rolled over, preserving the old tree's roots for ongoing validity proofs. A rollover can be performed once the rollover threshold  is met (default: 95% full).

**State vs Address Trees:**
- **State trees** have a separate `BatchedQueueAccount` for output operations (appending new leaves). Input operations (nullifying) use the integrated input queue on the tree account.
- **Address trees** have only an integrated input queue on the tree account - no separate output queue.

## ZKP Verification

Batch update operations require zero-knowledge proofs generated by the Light Protocol prover:

- **Prover Server:** `prover/server/` - Generates ZK proofs for batch operations
- **Prover Client:** `prover/client/` - Client libraries for requesting proofs
- **Batch Update Circuits:** `prover/server/prover/v2/` - Circuit definitions for batch append, batch update (nullify), and batch address append operations

## Dependencies

This crate relies on several Light Protocol libraries:

- **`light-bloom-filter`** - Bloom filter implementation for non-inclusion proofs
- **`light-hasher`** - Poseidon hash implementation for hash chains and tree operations
- **`light-verifier`** - ZKP verification for batch updates
- **`light-zero-copy`** - Zero-copy serialization for efficient account data access
- **`light-merkle-tree-metadata`** - Shared metadata structures for merkle trees
- **`light-compressed-account`** - Compressed account types and utilities
- **`light-account-checks`** - Account validation and discriminator checks

## Testing and Reference Implementations

**IndexedMerkleTree Reference Implementation:**
- **`light-merkle-tree-reference`** - Reference implementation of indexed Merkle trees (dev dependency)
- Source: `program-tests/merkle-tree/src/indexed.rs` - Canonical IndexedMerkleTree implementation used for generating constants and testing
- Used to generate constants like `ADDRESS_TREE_INIT_ROOT_40` in [`src/constants.rs`](../src/constants.rs)
- Initializes address trees with a single leaf: `H(0, HIGHEST_ADDRESS_PLUS_ONE)`

## Source Code Structure

**Core Account Types:**
- [`src/merkle_tree.rs`](../src/merkle_tree.rs) - `BatchedMerkleTreeAccount` (prove inclusion, nullify existing state, create new addresses)
- [`src/queue.rs`](../src/queue.rs) - `BatchedQueueAccount` (add new state (transaction outputs))
- [`src/batch.rs`](../src/batch.rs) - `Batch` state machine (Fill → Full → Inserted)
- [`src/queue_batch_metadata.rs`](../src/queue_batch_metadata.rs) - `QueueBatches` metadata

**Metadata and Configuration:**
- [`src/merkle_tree_metadata.rs`](../src/merkle_tree_metadata.rs) - `BatchedMerkleTreeMetadata` and account size calculations
- [`src/constants.rs`](../src/constants.rs) - Default configuration values

**ZKP Infrastructure:**
- `prover/server/` - Prover server that generates ZK proofs for batch operations
- `prover/client/` - Client libraries for requesting proofs
- `prover/server/prover/v2/` - Batch update circuit definitions (append, nullify, address append)

**Initialization:**
- [`src/initialize_state_tree.rs`](../src/initialize_state_tree.rs) - State tree initialization
- [`src/initialize_address_tree.rs`](../src/initialize_address_tree.rs) - Address tree initialization
- [`src/rollover_state_tree.rs`](../src/rollover_state_tree.rs) - State tree rollover
- [`src/rollover_address_tree.rs`](../src/rollover_address_tree.rs) - Address tree rollover

**Errors:**
- [`src/errors.rs`](../src/errors.rs) - `BatchedMerkleTreeError` enum with all error types

## Error Codes

All errors are defined in [`src/errors.rs`](../src/errors.rs) and map to u32 error codes (14301-14312 range):
- `BatchNotReady` (14301) - Batch is not ready to be inserted
- `BatchAlreadyInserted` (14302) - Batch is already inserted
- `TreeIsFull` (14310) - Batched Merkle tree reached capacity
- `NonInclusionCheckFailed` (14311) - Value exists in bloom filter
- `BloomFilterNotZeroed` (14312) - Bloom filter must be zeroed before reuse
- Additional errors from underlying libraries (hasher, zero-copy, verifier, etc.)
