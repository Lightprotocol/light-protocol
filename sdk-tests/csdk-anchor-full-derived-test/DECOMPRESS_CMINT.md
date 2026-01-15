# Decompress CMint via CPI

## Overview

Decompress a compressed mint to on-chain CMint PDA. **One CPI per mint** - the ctoken program's `DecompressMint` action does NOT support CPI context batching.

## Account Layout

```
remaining_accounts:
[0] ctoken_program
[1] light_system_program
[2] cpi_authority
[3] registered_program
[4] account_compression_authority
[5] account_compression_program
[6] state_tree
[7] input_queue
[8] output_queue
[9] mint_signer (for mint 1)
[10] cmint_pda (for mint 1)
```

## Client: Building Params

```rust
// Get compressed mint from indexer
let compressed_mint = rpc.get_compressed_account(hash, None).await;

// Get validity proof
let proof_result = rpc.get_validity_proof(vec![hash], vec![], None).await;

// Key: indices are relative to remaining_accounts
let packed_tree_info = proof_result.pack_tree_infos(&mut packed_accounts)
    .state_trees.unwrap().packed_tree_infos[0];

let params = DecompressCMintsParams {
    proof: ValidityProof(proof_result.proof.0),
    compressed_accounts: vec![CompressedMintAccountData {
        meta: CompressedAccountMetaNoLamportsNoAddress {
            tree_info: PackedStateTreeInfo {
                merkle_tree_pubkey_index: 6,  // state_tree index in remaining
                queue_pubkey_index: 7,         // input_queue index
                root_index: packed_tree_info.root_index,
                leaf_index: compressed_mint.leaf_index,
                prove_by_index: packed_tree_info.prove_by_index,
            },
            output_state_tree_index: 0,
        },
        data: CompressedMintVariant::Standard(CompressedMintTokenData {
            mint_seed_pubkey: mint_signer.pubkey(),
            compressed_mint_with_context: CompressedMintWithContext { ... },
            rent_payment: 16,
            write_top_up: 766,
        }),
    }],
    system_accounts_offset: 0,
};
```

## Footguns

1. **One mint per CPI** - `DecompressMint` blocks `first_set_context`/`set_context` flags. Each mint needs its own CPI call with its own proof.

2. **Index alignment** - Indices in params must match remaining_accounts positions exactly. Off-by-one = wrong account = failure.

3. **mint_signer is NOT a signer** - For decompress, `mint_signer.key()` is only used for PDA derivation. Don't mark it as signer in remaining_accounts.

4. **CMint PDA derivation** - Must match: `find_cmint_address(&mint_signer_pubkey)`. Verify on-chain before CPI.
