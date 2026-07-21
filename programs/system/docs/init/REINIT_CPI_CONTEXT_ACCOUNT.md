# ReInitCpiContextAccount

## Summary

| Field | Value |
|-------|-------|
| **Discriminator** | `[187, 147, 22, 142, 104, 180, 136, 190]` |
| **Enum** | `InstructionDiscriminator::ReInitCpiContextAccount` |
| **Path** | `programs/system/src/accounts/init_context_account.rs` |
| **Feature Gate** | `reinit` |

## Description

Migrates an existing CPI context account from version 1 to version 2. This instruction reads the associated Merkle tree from the existing account, resizes the account to the new size, and reinitializes with version 2 format.

**Critical:** This is an in-place migration. The account must be owned by Light System Program and have the V1 discriminator.

### Use Cases
- Upgrading legacy CPI context accounts to the new format
- Migrating accounts after protocol upgrades
- Preparing existing accounts for enhanced CPI context features

### State Changes
- Account is resized from 20,488 bytes (V1) to 14,020 bytes (V2)
- Discriminator is updated from V1 `[22, 20, 149, 218, 74, 204, 128, 166]` to V2 `[34, 184, 183, 14, 100, 80, 183, 124]`
- All account data is zeroed except the associated Merkle tree (preserved)
- Vector capacities are initialized with default values

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
| 0 | `cpi_context_account` | No | Yes | Existing version 1 CPI context account to migrate |

### Account Validations

**cpi_context_account (index 0):**
- Must be owned by Light System Program
- Must have discriminator `[22, 20, 149, 218, 74, 204, 128, 166]` (V1)
- Must be writable
- Will be resized from 20,488 bytes to 14,020 bytes

---

## Instruction Logic

### Step 1: Validate Ownership
```rust
check_owner(&crate::ID, cpi_context_account)?;
```

### Step 2: Read Associated Merkle Tree
```rust
let associated_merkle_tree = {
    let data = cpi_context_account.try_borrow_data()?;
    CpiContextAccount::deserialize(&mut &data[8..])?.associated_merkle_tree
};
```

Must happen before resize to preserve the merkle tree reference.

### Step 3: Resize Account
```rust
cpi_context_account.resize(DEFAULT_CPI_CONTEXT_ACCOUNT_SIZE_V2 as usize)?;
```

V2 is smaller than V1, so no additional rent payment required.

### Step 4: Reinitialize Account
```rust
let params = CpiContextAccountInitParams::new(associated_merkle_tree);
cpi_context_account_new::<true>(cpi_context_account, params)?;
```

Writes V2 discriminator, preserves associated_merkle_tree, initializes vector capacities with defaults.

---

## Errors

| Code | Error | Cause |
|------|-------|-------|
| - | `NotEnoughAccountKeys` | No accounts provided |
| - | `IllegalOwner` | Account not owned by Light System Program |
| - | `BorshIoError` | Failed to deserialize V1 account data |
| 6025 | `InvalidCpiContextDiscriminator` | Account discriminator is not V1 (may already be migrated) |

---

## Version Differences

### Version 1 (Legacy)

```rust
// Discriminator: [22, 20, 149, 218, 74, 204, 128, 166]
pub struct CpiContextAccount {
    pub fee_payer: Pubkey,
    pub associated_merkle_tree: Pubkey,
}
```

**Size:** 20,488 bytes

### Version 2 (Current)

```rust
// Discriminator: [34, 184, 183, 14, 100, 80, 183, 124]
pub struct ZCpiContextAccount2 {
    pub fee_payer: Pubkey,
    pub associated_merkle_tree: Pubkey,
    _associated_queue: Pubkey,
    _place_holder_bytes: [u8; 32],
    pub new_addresses: ZeroCopyVecU8<CpiContextNewAddressParamsAssignedPacked>,
    pub readonly_addresses: ZeroCopyVecU8<ZPackedReadOnlyAddress>,
    pub readonly_accounts: ZeroCopyVecU8<ZPackedReadOnlyCompressedAccount>,
    pub in_accounts: ZeroCopyVecU8<CpiContextInAccount>,
    pub out_accounts: ZeroCopyVecU8<CpiContextOutAccount>,
    // ...
}
```

**Size:** 14,020 bytes (31% reduction)

**Benefits:**
- Zero-copy deserialization (faster)
- Structured vector management
- Smaller size saves rent

---

## Related Documentation

- [ACCOUNTS.md](../ACCOUNTS.md) - CpiContextAccount layout details
- [CPI_CONTEXT.md](../CPI_CONTEXT.md) - How CPI context is used
- [INIT_CPI_CONTEXT_ACCOUNT.md](INIT_CPI_CONTEXT_ACCOUNT.md) - Creating new version 2 accounts
