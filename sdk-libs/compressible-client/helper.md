# Compressed Account Client Helper

## The Problem

Building remaining accounts is verbose and error-prone:

```rust
// Current: 10+ lines of boilerplate every time
let mut packed = PackedAccounts::default();
let system_config = match cpi_context {
    Some(ctx) => SystemAccountMetaConfig::new_with_cpi_context(program_id, ctx),
    None => SystemAccountMetaConfig::new(program_id),
};
packed.add_system_accounts_v2(system_config)?;
let output_queue = tree_info.next_tree_info.as_ref().map(|n| n.queue).unwrap_or(tree_info.queue);
let output_tree_index = packed.insert_or_get(output_queue);
let packed_trees = proof.pack_tree_infos(&mut packed);
let (remaining_accounts, system_offset, _) = packed.to_account_metas();
```

## The Solution

One function:

```rust
pub struct PackedProofResult {
    /// Remaining accounts to append to your instruction's accounts.
    pub remaining_accounts: Vec<AccountMeta>,
    /// Packed tree infos. Use `.address_trees` or `.state_trees` as needed.
    pub packed_tree_infos: PackedTreeInfos,
    /// Index of output tree in remaining accounts.
    pub output_tree_index: u8,
    /// Offset where system accounts start (if needed).
    pub system_accounts_offset: u8,
}

/// Packs validity proof into remaining accounts.
///
/// # Arguments
/// - `program_id`: Your program ID
/// - `proof`: From `get_validity_proof()`
/// - `output_tree`: From `get_random_state_tree_info()`
/// - `cpi_context`: `tree_info.cpi_context` when mixing PDAs+tokens, else `None`
pub fn pack_proof(
    program_id: &Pubkey,
    proof: ValidityProofWithContext,
    output_tree: &TreeInfo,
    cpi_context: Option<Pubkey>,
) -> Result<PackedProofResult, PackError>;
```

## Full Flow

```rust
// 1. Derive addresses (use existing functions)
let user_addr = derive_address(&user_pda.to_bytes(), &tree.to_bytes(), &program_id.to_bytes());
let mint_addr = derive_cmint_compressed_address(&mint_signer, &tree);

// 2. Get proof + output tree
let proof = rpc.get_validity_proof(
    vec![],  // existing hashes (empty for new accounts)
    vec![    // new addresses
        AddressWithTree { address: user_addr, tree },
        AddressWithTree { address: mint_addr, tree },
    ],
    None,
).await?.value;
let output_tree = rpc.get_random_state_tree_info()?;

// 3. Pack (the helper)
let packed = pack_proof(
    &program_id,
    proof.clone(),
    &output_tree,
    output_tree.cpi_context,  // Some for mixed PDA+token, None for PDA-only
)?;

// 4. Build instruction
let ix = Instruction {
    program_id,
    accounts: [my_accounts.to_account_metas(None), packed.remaining_accounts].concat(),
    data: MyInstruction {
        proof: proof.proof,
        address_tree_infos: packed.packed_tree_infos.address_trees,
        output_tree_index: packed.output_tree_index,
        // ...
    }.data(),
};
```

## When to use CPI context

```
PDA-only tx         → cpi_context: None
Token-only tx       → cpi_context: None
Mixed PDA + token   → cpi_context: tree_info.cpi_context (Option<Pubkey>)
```

## Errors

```rust
#[derive(Debug, Error)]
pub enum PackError {
    #[error("Failed to add system accounts: {0}")]
    SystemAccounts(#[from] LightSdkError),
}
```

## Files

| File          | Contents                                         |
| ------------- | ------------------------------------------------ |
| `src/pack.rs` | `pack_proof()`, `PackedProofResult`, `PackError` |
| `src/lib.rs`  | Re-export                                        |
