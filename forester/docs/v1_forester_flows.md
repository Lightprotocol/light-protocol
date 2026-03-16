# Forester V1 Flows (PR: v2 Nullify + Blockhash)

## 1. Transaction Send Flow (Blockhash)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                        send_batched_transactions                                 │
└─────────────────────────────────────────────────────────────────────────────────┘

  ┌──────────────────────────────────┐
  │ prepare_batch_prerequisites       │
  │ - fetch queue items              │
  │ - single RPC: blockhash +        │
  │   priority_fee (same connection)  │
  │ - PreparedBatchData:             │
  │   recent_blockhash               │
  │   last_valid_block_height        │
  └──────────────┬───────────────────┘
                 │
                 ▼
  ┌──────────────────────────────────┐
  │ for each work_chunk (100 items)   │
  └──────────────┬───────────────────┘
                 │
    ┌────────────┴────────────┐
    │ elapsed > 30s?           │
    │   YES → refresh blockhash│
    │   (pool.get_connection   │
    │    → rpc.get_latest_     │
    │      blockhash)          │
    │   NO  → keep current    │
    └────────────┬────────────┘
                 │
                 ▼
  ┌──────────────────────────────────┐
  │ build_signed_transaction_batch    │
  │ (recent_blockhash,               │
  │  last_valid_block_height)        │
  │ → (txs, chunk_last_valid_        │
  │    block_height)                  │
  └──────────────┬───────────────────┘
                 │
                 ▼
  ┌──────────────────────────────────┐
  │ execute_transaction_chunk_sending │
  │ PreparedTransaction::legacy(     │
  │   tx, chunk_last_valid_block_    │
  │   height)                        │
  │ - send + confirm                 │
  │ - blockhash expiry check via     │
  │   last_valid_block_height        │
  └──────────────────────────────────┘

  No refetch-before-send. No re-sign.
```

## 2. State Nullify Instruction Flow (Legacy vs v2)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│ Registry: nullify instruction paths                                               │
└─────────────────────────────────────────────────────────────────────────────────┘

  LEGACY (proof in ix data)                    v2 (proof in remaining_accounts)
  ───────────────────────                     ────────────────────────────────────

  create_nullify_instruction()                 create_nullify_with_proof_accounts_instruction()
       │                                              │
       │ ix data: [change_log, queue_idx,             │ ix data: [change_log, queue_idx,
       │           leaf_idx, proofs[16][32]]           │           leaf_idx]  (no proofs)
       │                                              │
       │ remaining_accounts: standard                  │ remaining_accounts: 16 proof
       │ (authority, merkle_tree, queue...)            │ account pubkeys (key = node bytes)
       │                                              │
       ▼                                              ▼
  process_nullify()                            nullify_2 instruction
  (proofs from ix data)                        - validate: 1 change, 1 queue, 1 index
                                               - validate: exactly 16 proof accounts
                                               - extract_proof_nodes_from_remaining_accounts
                                               - process_nullify(..., vec![proof_nodes])

  Forester V1 uses nullify_2 only (create_nullify_2_instruction).
```

## 3. Forester V1 State Nullify Pairing Flow

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│ build_instruction_batches (state nullify path)                                    │
└─────────────────────────────────────────────────────────────────────────────────┘

  fetch_proofs_and_create_instructions
       │
       │ For each state item:
       │   create_nullify_with_proof_accounts_instruction (v2)
       │   → StateNullifyInstruction { instruction, proof_nodes, leaf_index }
       │
       ▼
  ┌─────────────────────────────────────────────────────────────────────────────┐
  │ allow_pairing?                                                               │
  │   batch_size >= 2  AND  should_attempt_pairing()                             │
  └─────────────────────────────────────────────────────────────────────────────┘
       │
       │ should_attempt_pairing checks:
       │   - pair_candidates = n*(n-1)/2 <= 2000 (MAX_PAIR_CANDIDATES)
       │   - state_nullify_count <= 96 (MAX_PAIRING_INSTRUCTIONS)
       │   - remaining_blocks = last_valid - current > 25 (MIN_REMAINING_BLOCKS_FOR_PAIRING)
       │
       ├── NO  → each nullify → 1 tx (no pairing)
       │
       └── YES → pair_state_nullify_batches
                     │
                     │ For each pair (i,j):
                     │   - pair_fits_transaction_size(ix_i, ix_j)? (serialized <= 1232)
                     │   - weight = 10000 + proof_overlap_count
                     │
                     │ Max-cardinality matching (mwmatching)
                     │ - prioritize number of pairs
                     │ - then maximize proof overlap (fewer unique accounts)
                     │
                     ▼
                 Output: Vec<Vec<Instruction>>
                 - paired: [ix_a, ix_b] in one tx
                 - unpaired: [ix] in one tx

  Address updates: no pairing, chunked by batch_size only.
```

## 4. End-to-End Forester V1 State Tree Flow

```
  Queue (state nullifier)     Indexer (proofs)
         │                            │
         └──────────┬─────────────────┘
                    │
                    ▼
  prepare_batch_prerequisites
  - queue items
  - blockhash + last_valid_block_height
  - priority_fee
                    │
                    ▼
  for chunk in work_items.chunks(100):
      refresh blockhash if 30s elapsed
                    │
                    ▼
  build_signed_transaction_batch
      │
      ├─ fetch_proofs_and_create_instructions
      │     - state: v2 nullify ix (proof in remaining_accounts)
      │     - address: update ix
      │
      ├─ build_instruction_batches
      │     - address: chunk by batch_size
      │     - state nullify: pair if allow_pairing else 1-per-tx
      │
      └─ create_smart_transaction per batch
                    │
                    ▼
  execute_transaction_chunk_sending
  - PreparedTransaction::legacy(tx, chunk_last_valid_block_height)
  - send + confirm with blockhash expiry check
```

