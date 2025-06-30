# SDK Token Test Debugging Guide

This document contains debugging findings for the Light Protocol SDK Token Test program, specifically for implementing the 4 invocations instruction and compressed escrow PDA creation.

## Error Code Reference

### Light SDK Errors

| Error Code | Hex Code | Error Name | Description |
|------------|----------|------------|-------------|
| 16031 | 0x3e9f | `CpiAccountsIndexOutOfBounds` | Trying to access an account index that doesn't exist in the account list |
| 16032 | 0x3ea0 | `InvalidCpiContextAccount` | CPI context account is invalid |
| 16033 | 0x3ea1 | `InvalidSolPoolPdaAccount` | Sol pool PDA account is invalid |

### Light System Program Errors

| Error Code | Hex Code | Error Name | Description |
|------------|----------|------------|-------------|
| 6017 | 0x1781 | `ProofIsNone` | Proof is required but not provided |
| 6018 | 0x1782 | `ProofIsSome` | Proof provided when not expected |
| 6019 | 0x1783 | `EmptyInputs` | Empty inputs provided |
| 6020 | 0x1784 | `CpiContextAccountUndefined` | CPI context account is not properly defined |
| 6021 | 0x1785 | `CpiContextEmpty` | CPI context is empty |
| 6022 | 0x1786 | `CpiContextMissing` | CPI context is missing |
| 6023 | 0x1787 | `DecompressionRecipientDefined` | Decompression recipient wrongly defined |

## Common Issues and Solutions

### 1. `CpiAccountsIndexOutOfBounds` (Error 16031)

**Problem**: Attempting to access an account at an index that doesn't exist in the accounts array.

**Solution**: Ensure all required accounts are properly added to the `PackedAccounts` structure before calling `to_account_metas()`.

**Example Fix**:
```rust
// ❌ Wrong - missing signer account
let (accounts, _, _) = remaining_accounts.to_account_metas();

// ✅ Correct - add signer first
remaining_accounts.add_pre_accounts_signer_mut(payer.pubkey());
let (accounts, _, _) = remaining_accounts.to_account_metas();
```

### 2. Privilege Escalation Error

**Problem**: "Cross-program invocation with unauthorized signer or writable account"

**Root Cause**: Manually adding signer accounts to instruction accounts array instead of using the PackedAccounts structure.

**Solution**: Use `add_pre_accounts_signer_mut()` instead of manually prepending accounts.

**Example Fix**:
```rust
// ❌ Wrong - manual signer addition
let instruction = Instruction {
    program_id: sdk_token_test::ID,
    accounts: [vec![AccountMeta::new(payer.pubkey(), true)], accounts].concat(),
    data: instruction_data.data(),
};

// ✅ Correct - use PackedAccounts
remaining_accounts.add_pre_accounts_signer_mut(payer.pubkey());
let (accounts, _, _) = remaining_accounts.to_account_metas();
let instruction = Instruction {
    program_id: sdk_token_test::ID,
    accounts,
    data: instruction_data.data(),
};
```

### 3. Account Structure Mismatch

**Problem**: Using wrong account structure (e.g., `GenericWithAuthority` vs `Generic`)

**Root Cause**: `GenericWithAuthority` expects 2 accounts (`signer` + `authority`), while `Generic` expects 1 account (`signer` only).

**Solution**: Choose the correct account structure based on your needs.

**Example**:
```rust
// For PDA creation - only need signer
pub fn create_escrow_pda<'info>(
    ctx: Context<'_, '_, '_, 'info, Generic<'info>>, // ✅ Correct
    // ...
) -> Result<()>

// For operations requiring authority
pub fn four_invokes<'info>(
    ctx: Context<'_, '_, '_, 'info, GenericWithAuthority<'info>>, // ✅ Correct
    // ...
) -> Result<()>
```

### 4. `CpiContextAccountUndefined` (Error 6020)

**Problem**: CPI context account is not properly defined or provided to the Light System program.

**Root Cause**: This error occurs when trying to use CPI context functionality without properly providing the CPI context account. The error comes from `process_cpi_context.rs:52` in the Light System program.

**Common Causes**:
- Using functions that expect CPI context (like `process_create_compressed_account`) when you don't actually need CPI context
- Missing CPI context configuration in `SystemAccountMetaConfig`
- Wrong CPI context account in tree info
- Reusing code that was designed for CPI context operations in non-CPI context scenarios

**Understanding CPI Context**:
CPI context is used to optimize transactions that need multiple cross-program invocations with compressed accounts. It allows:
- Sending only one proof for the entire instruction instead of multiple proofs
- Caching signer checks across multiple CPIs
- Combining instruction data from different programs

**Example Flow**:
1. First invocation (e.g., token program): Performs signer checks, caches in CPI context, returns without state transition
2. Second invocation (e.g., PDA program): Reads CPI context, combines instruction data, executes with combined proof
3. Subsequent invocations can add more data to the context
4. Final invocation executes all accumulated operations

**Solutions**:

**Option 1 - Don't use CPI context (Recommended for simple operations)**:
```rust
// ✅ For simple operations without cross-program complexity
let cpi_inputs = CpiInputs {
    proof,
    account_infos: Some(vec![my_compressed_account.to_account_info().unwrap()]),
    new_addresses: Some(vec![new_address_params]),
    cpi_context: None, // ← Key: Set to None
    ..Default::default()
};
cpi_inputs.invoke_light_system_program(cpi_accounts)
```

**Option 2 - Proper CPI context setup (For complex cross-program operations)**:
```rust
// ✅ Only use when you actually need CPI context optimization
let tree_info = rpc.get_random_state_tree_info().unwrap();
let config = SystemAccountMetaConfig::new_with_cpi_context(
    program_id,
    tree_info.cpi_context.unwrap(), // Ensure CPI context exists
);
remaining_accounts.add_system_accounts(config);
```

## Best Practices

### Account Management
1. Always use `PackedAccounts` for account management
2. Add signer accounts using `add_pre_accounts_signer_mut()`
3. Add system accounts using `add_system_accounts()` with proper config
4. Never manually manipulate the accounts array

### CPI Context
1. Always check that `tree_info.cpi_context` is `Some()` before using
2. Use `new_with_cpi_context()` for operations requiring CPI context
3. Ensure CPI context configuration matches the instruction requirements

### Error Debugging
1. Convert hex error codes to decimal for easier lookup
2. Check both Light SDK and Light System program error codes
3. Use the error code tables above for quick reference

## Testing

### Compress Function Pattern
The working compress function follows this pattern:
```rust
async fn compress_spl_tokens(
    rpc: &mut impl Rpc,
    payer: &Keypair,
    recipient: Pubkey,
    mint: Pubkey,
    amount: u64,
    token_account: Pubkey,
) -> Result<Signature, RpcError> {
    let mut remaining_accounts = PackedAccounts::default();
    let token_pool_pda = get_token_pool_pda(&mint);
    let config = TokenAccountsMetaConfig::compress_client(
        token_pool_pda,
        token_account,
        SPL_TOKEN_PROGRAM_ID.into(),
    );
    remaining_accounts.add_pre_accounts_signer_mut(payer.pubkey());
    let metas = get_transfer_instruction_account_metas(config);
    remaining_accounts.add_pre_accounts_metas(metas.as_slice());

    let output_tree_index = rpc
        .get_random_state_tree_info()
        .unwrap()
        .pack_output_tree_index(&mut remaining_accounts)
        .unwrap();

    let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: sdk_token_test::ID,
        accounts: remaining_accounts,
        data: sdk_token_test::instruction::CompressTokens {
            output_tree_index,
            recipient,
            mint,
            amount,
        }
        .data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}
```

### Test Structure
A typical test follows this flow:
1. **Setup**: Create mints and token accounts
2. **Compress**: Compress tokens using the compress function
3. **Create PDA**: Create compressed escrow PDA
4. **Execute**: Run the 4 invocations instruction

This debugging guide should help future developers avoid common pitfalls when working with Light Protocol compressed accounts and CPI operations.