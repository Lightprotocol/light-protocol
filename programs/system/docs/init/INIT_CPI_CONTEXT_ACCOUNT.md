# InitializeCpiContextAccount

## Summary

| Field | Value |
|-------|-------|
| **Discriminator** | `[233, 112, 71, 66, 121, 33, 178, 188]` |
| **Enum** | `InstructionDiscriminator::InitializeCpiContextAccount` |
| **Path** | `programs/system/src/accounts/init_context_account.rs` |

## Description

Initializes a new CPI context account (version 2) for use in multi-program compressed account transactions. The account is associated with a specific state Merkle tree and allocated with default capacity parameters.

### Use Cases
- Setting up a CPI context account for a new Merkle tree
- Preparing for multi-program transactions that share a single ZK proof
- Creating dedicated context accounts for specific applications

### State Changes
- Account data is initialized with version 2 discriminator
- Associated Merkle tree is set
- Default capacity parameters are applied

---

## Instruction Data

This instruction takes no additional data beyond the discriminator.

```rust
// Instruction data layout:
// [0..8]: Discriminator
```

**Total size:** 8 bytes

---

## Accounts

| Index | Name | Signer | Writable | Description |
|-------|------|--------|----------|-------------|
| 0 | `fee_payer` | Yes | Yes | Pays for account creation |
| 1 | `cpi_context_account` | No | Yes | Account to initialize (must be pre-allocated) |
| 2 | `associated_merkle_tree` | No | No | State Merkle tree to associate with |

### Account Validations

**fee_payer (index 0):**
- Must be a signer
- Must be writable

**cpi_context_account (index 1):**
- Must be pre-allocated with exactly 14020 bytes (`DEFAULT_CPI_CONTEXT_ACCOUNT_SIZE_V2`)
- Must be owned by Light System Program
- Discriminator must be zero (uninitialized)

**associated_merkle_tree (index 2):**
- Must be owned by Account Compression Program
- Discriminator must match state or batched Merkle tree

---

## Instruction Logic

### Step 1: Validate Accounts
```rust
let ctx = InitializeCpiContextAccount::from_account_infos(accounts)?;
```

### Step 2: Initialize Account
```rust
let params = CpiContextAccountInitParams::new(*ctx.associated_merkle_tree.key());
cpi_context_account_new::<false>(ctx.cpi_context_account, params)?;
```

Initialization:
1. Write version 2 discriminator: `[34, 184, 183, 14, 100, 80, 183, 124]`
2. Set fee_payer to zero (will be set during operation)
3. Set associated_merkle_tree
4. Set up vector capacities with defaults:
   - `new_addresses_len`: 10
   - `readonly_addresses_len`: 10
   - `readonly_accounts_len`: 10
   - `in_accounts_len`: 20
   - `out_accounts_len`: 30

---

## Errors

| Code | Error | Cause |
|------|-------|-------|
| - | `NotEnoughAccountKeys` | Less than 3 accounts provided |
| - | `MissingRequiredSignature` | fee_payer is not a signer |
| - | `IllegalOwner` | associated_merkle_tree not owned by account-compression program OR cpi_context_account not owned by Light System Program |
| 6042 | `StateMerkleTreeAccountDiscriminatorMismatch` | associated_merkle_tree discriminator doesn't match state or batched Merkle tree |
| 6055 | `InvalidCpiContextOwner` | cpi_context_account not owned by Light System Program |
| 6056 | `InvalidCpiContextDiscriminator` | cpi_context_account discriminator is not zero (already initialized) |

---

## Related Documentation

- [ACCOUNTS.md](../ACCOUNTS.md) - CpiContextAccount layout details
- [CPI_CONTEXT.md](../CPI_CONTEXT.md) - How CPI context is used
- [REINIT_CPI_CONTEXT_ACCOUNT.md](REINIT_CPI_CONTEXT_ACCOUNT.md) - Migration from version 1
