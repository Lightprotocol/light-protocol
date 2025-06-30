# SDK Token Test Debugging Guide

## Error Code Reference

| Error Code | Error Name | Description | Common Fix |
|------------|------------|-------------|------------|
| 16031 | `CpiAccountsIndexOutOfBounds` | Missing account in accounts array | Add signer with `add_pre_accounts_signer_mut()` |
| 6020 | `CpiContextAccountUndefined` | CPI context expected but not provided | Set `cpi_context: None` for simple operations |

### Light System Program Errors (Full Reference)
| 6017 | `ProofIsNone` | 6018 | `ProofIsSome` | 6019 | `EmptyInputs` | 6020 | `CpiContextAccountUndefined` |
| 6021 | `CpiContextEmpty` | 6022 | `CpiContextMissing` | 6023 | `DecompressionRecipientDefined` |

## Common Issues and Solutions

### 1. `CpiAccountsIndexOutOfBounds` (Error 16031)
Missing signer account. **Fix**: `remaining_accounts.add_pre_accounts_signer_mut(payer.pubkey())`

### 2. Privilege Escalation Error
Manually adding accounts instead of using PackedAccounts. **Fix**: Use `add_pre_accounts_signer_mut()` instead of manual account concatenation.

### 3. Account Structure Mismatch
Wrong context type. **Fix**: Use `Generic<'info>` for single signer, `GenericWithAuthority<'info>` for signer + authority.

### 4. `CpiContextAccountUndefined` (Error 6020)
**Root Cause**: Using functions designed for CPI context when you don't need it.

**CPI Context Purpose**: Optimize multi-program transactions by using one proof instead of multiple. Flow:
1. First program: Cache signer checks in CPI context
2. Second program: Read context, combine data, execute with single proof

**Solutions**:
```rust
// ✅ Simple operations - no CPI context
let cpi_inputs = CpiInputs {
    proof,
    account_infos: Some(vec![account.to_account_info().unwrap()]),
    new_addresses: Some(vec![new_address_params]),
    cpi_context: None, // ← Key
    ..Default::default()
};

// ✅ Complex multi-program operations - use CPI context  
let config = SystemAccountMetaConfig::new_with_cpi_context(program_id, cpi_context_account);
```

### 5. Avoid Complex Function Reuse
**Problem**: Functions like `process_create_compressed_account` expect CPI context setup.

**Fix**: Use direct Light SDK approach:
```rust
// ❌ Complex function with CPI context dependency
process_create_compressed_account(...)

// ✅ Direct approach
let mut account = LightAccount::<'_, CompressedEscrowPda>::new_init(&crate::ID, Some(address), tree_index);
account.amount = amount;
account.owner = *cpi_accounts.fee_payer().key;
let cpi_inputs = CpiInputs { proof, account_infos: Some(vec![account.to_account_info().unwrap()]), cpi_context: None, ..Default::default() };
cpi_inputs.invoke_light_system_program(cpi_accounts)
```

### 6. Critical Four Invokes Implementation Learnings

**CompressInputs Structure for CPI Context Operations**:
```rust
let compress_inputs = CompressInputs {
    fee_payer: *cpi_accounts.fee_payer().key,
    authority: *cpi_accounts.fee_payer().key,
    mint,
    recipient,
    sender_token_account: *remaining_accounts[0].key, // ← Use remaining_accounts index
    amount,
    output_tree_index,
    // ❌ Wrong: output_queue_pubkey: *cpi_accounts.tree_accounts().unwrap()[0].key,
    token_pool_pda: *remaining_accounts[1].key, // ← From remaining_accounts
    transfer_config: Some(TransferConfig {
        cpi_context: Some(CompressedCpiContext {
            set_context: true,
            first_set_context: true,
            cpi_context_account_index: 0,
        }),
        cpi_context_pubkey: Some(cpi_context_pubkey),
        ..Default::default()
    }),
    spl_token_program: *remaining_accounts[2].key, // ← SPL_TOKEN_PROGRAM_ID
    tree_accounts: cpi_accounts.tree_pubkeys().unwrap(), // ← From CPI accounts
};
```

**Critical Account Ordering for Four Invokes**:
```rust
// Test setup - exact order matters for remaining_accounts indices
remaining_accounts.add_pre_accounts_signer_mut(payer.pubkey());
// Remaining accounts 0 - compression token account
remaining_accounts.add_pre_accounts_meta(AccountMeta::new(compression_token_account, false));
// Remaining accounts 1 - token pool PDA
remaining_accounts.add_pre_accounts_meta(AccountMeta::new(token_pool_pda1, false));
// Remaining accounts 2 - SPL token program
remaining_accounts.add_pre_accounts_meta(AccountMeta::new(SPL_TOKEN_PROGRAM_ID.into(), false));
// Remaining accounts 3 - compressed token program
remaining_accounts.add_pre_accounts_meta(AccountMeta::new(compressed_token_program, false));
// Remaining accounts 4 - CPI authority PDA
remaining_accounts.add_pre_accounts_meta(AccountMeta::new(cpi_authority_pda, false));
```

**Validity Proof and Tree Info Management**:
```rust
// Get escrow account directly by address (more efficient)
let escrow_account = rpc.get_compressed_account(escrow_address, None).await?.value;

// Pack tree infos BEFORE constructing TokenAccountMeta
let packed_tree_info = rpc_result.pack_tree_infos(&mut remaining_accounts);

// Use correct tree info indices for each compressed account
let mint2_tree_info = packed_tree_info.state_trees.as_ref().unwrap().packed_tree_infos[1];
let mint3_tree_info = packed_tree_info.state_trees.as_ref().unwrap().packed_tree_infos[2];
let escrow_tree_info = packed_tree_info.state_trees.as_ref().unwrap().packed_tree_infos[0];
```

**System Accounts Start Offset**:
```rust
// Use the actual offset returned by to_account_metas()
let (accounts, system_accounts_start_offset, _) = remaining_accounts.to_account_metas();
// Pass this offset to the instruction
system_accounts_start_offset: system_accounts_start_offset as u8,
```

## Best Practices

### CPI Context Decision
- **Use**: Multi-program transactions with compressed accounts (saves proofs)
- **Avoid**: Simple single-program operations (PDA creation, basic transfers)

### Account Management
- Use `PackedAccounts` and `add_pre_accounts_signer_mut()`
- Choose `Generic<'info>` (1 account) vs `GenericWithAuthority<'info>` (2 accounts)
- Set `cpi_context: None` for simple operations

### Working Patterns
```rust
// Compress tokens pattern
let mut remaining_accounts = PackedAccounts::default();
remaining_accounts.add_pre_accounts_signer_mut(payer.pubkey());
let metas = get_transfer_instruction_account_metas(config);
remaining_accounts.add_pre_accounts_metas(metas.as_slice());
let output_tree_index = rpc.get_random_state_tree_info().unwrap().pack_output_tree_index(&mut remaining_accounts).unwrap();

// Test flow: Setup → Compress → Create PDA → Execute
```

## Implementation Status

### ✅ Working Features
1. **Basic PDA Creation**: `create_escrow_pda` instruction works correctly
2. **Token Compression**: Individual token compression operations work  
3. **Four Invokes Instruction**: Complete CPI context implementation working
   - Account structure: Uses `Generic<'info>` (single signer)
   - CPI context: Proper multi-program proof optimization 
   - Token accounts: Correct account ordering and tree info management
   - Compress CPI: Working with proper `CompressInputs` structure
   - Transfer CPI: Custom `transfer_tokens_with_cpi_context` wrapper replaces `transfer_tokens_to_escrow_pda`
4. **Error Handling**: Comprehensive error code documentation and fixes

### Key Implementation Success
The `four_invokes` instruction successfully demonstrates the complete CPI context pattern for Light Protocol, enabling:
- **Single Proof Optimization**: One validity proof for multiple compressed account operations
- **Cross-Program Integration**: Token program + system program coordination  
- **Production Ready**: Complete account setup and tree info management
- **Custom Transfer Wrapper**: Purpose-built transfer function for four invokes instruction