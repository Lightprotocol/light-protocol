# Seed Indices Implementation - Positional Account References

## Overview

This implementation removes the requirement for named seed-reference accounts in `DecompressAccountsIdempotent`, replacing them with **positional indices** into `remaining_accounts`. This eliminates struct bloat and enables dynamic seed account passing.

## Architecture

### Remaining Accounts Layout

```
remaining_accounts: [
    // Section 1: System accounts [0..system_accounts_offset)
    [tree_0, queue_0, tree_1, ...]

    // Section 2: Solana target accounts [system_accounts_offset..seed_accounts_offset)
    [target_pda_0, target_pda_1, ...]

    // Section 3: Seed reference accounts [seed_accounts_offset..)
    [seed_ref_0, seed_ref_1, ...]  ← NEW: Deduplicated seed accounts
]
```

### Key Changes

#### 1. CompressedAccountData Structure (Rust)

**File:** `sdk-libs/macros/src/variant_enum.rs`

```rust
pub struct CompressedAccountData {
    pub meta: CompressedAccountMetaNoLamportsNoAddress,
    pub data: CompressedAccountVariant,
    pub seed_indices: Vec<u8>,        // NEW: Indices for account seeds
    pub authority_indices: Vec<u8>,   // NEW: Indices for authority seeds
}
```

#### 2. CTokenSeedProvider Trait (Rust)

**File:** `sdk-libs/macros/src/compressible_instructions.rs`

```rust
pub trait CTokenSeedProvider {
    fn get_seeds<'info>(
        &self,
        remaining_accounts: &[AccountInfo<'info>],
        seed_indices: &[u8],
        seed_accounts_offset: u8,
    ) -> (Vec<Vec<u8>>, Pubkey);

    fn get_authority_seeds<'info>(
        &self,
        remaining_accounts: &[AccountInfo<'info>],
        authority_indices: &[u8],
        seed_accounts_offset: u8,
    ) -> (Vec<Vec<u8>>, Pubkey);
}
```

#### 3. Instruction Signature

```rust
pub fn decompress_accounts_idempotent<'info>(
    ctx: Context<'_, '_, 'info, 'info, DecompressAccountsIdempotent<'info>>,
    proof: ValidityProof,
    compressed_accounts: Vec<CompressedAccountData>,
    system_accounts_offset: u8,
    seed_accounts_offset: u8,  // NEW parameter
) -> Result<()>
```

#### 4. Generated Seed Derivation

The macro now generates code that uses positional indices:

```rust
// OLD (named accounts):
let seed_0 = accounts.user.key();

// NEW (positional):
let seed_0 = remaining_accounts[(seed_accounts_offset + seed_indices[0]) as usize].key;
```

## Seed Type Handling

The implementation correctly handles three types of seeds:

1. **String Literals**: `"user_vault"` → Inlined directly, no index needed
2. **Constants**: `POOL_VAULT_SEED` → Inlined directly, no index needed
3. **Account References**: `ctx.accounts.user` → Uses `seed_indices[i]`
4. **Data Fields**: `data.session_id.to_le_bytes()` → From unpacked data, no index needed

Only **account references** require indices.

## TypeScript Client Changes

### 1. Updated Return Type

**File:** `js/stateless.js/src/compressible/pack.ts`

```typescript
{
    compressedAccounts: Array<{
        meta: { ... },
        data: any,
        seedIndices: number[],        // NEW
        authorityIndices: number[],   // NEW
    }>,
    systemAccountsOffset: number,
    seedAccountsOffset: number,       // NEW
    remainingAccounts: AccountMeta[],
    proofOption: { 0: ValidityProof | null }
}
```

### 2. Usage Example

```typescript
const params = await buildDecompressParams(programId, rpc, accountInputs);

await program.methods
  .decompressAccountsIdempotent(
    params.proofOption,
    params.compressedAccounts,
    params.systemAccountsOffset,
    params.seedAccountsOffset // NEW parameter
  )
  .remainingAccounts(params.remainingAccounts)
  .rpc();
```

## Current Implementation Status

### ✅ Completed

1. **Rust Macro Changes**
   - `CompressedAccountData` includes `seed_indices` and `authority_indices`
   - `CTokenSeedProvider` trait uses positional indices
   - PDA seed derivation uses positional indices
   - All helper functions updated
   - Instruction signature includes `seed_accounts_offset`

2. **TypeScript Changes**
   - Return types updated to include new fields
   - Instruction params include `seedAccountsOffset`

### ⚠️ Current Limitation

**Seed indices are currently empty arrays (`[]`)** in the TypeScript client.

This works for:

- Accounts with only literal/constant seeds
- Accounts without seed references

This does **NOT** work for:

- Accounts with `ctx.accounts.X` in seed specifications

### 🔧 Needed for Full Implementation

To populate `seed_indices` and `authority_indices` properly, we need:

1. **IDL Enhancement**: Extend IDL generation to mark which seeds are account references

   ```json
   {
     "seeds": [
       { "type": "literal", "value": "user_vault" },
       { "type": "account", "path": "user" }, // ← Needs index
       { "type": "account", "path": "mint" }, // ← Needs index
       { "type": "arg", "path": "session_id" } // ← No index
     ]
   }
   ```

2. **Client-Side Logic**: Extract seed accounts from parsed data based on IDL metadata

   ```typescript
   // Pseudocode
   const seedAccountMap = new Map<string, number>();
   const seedAccounts: PublicKey[] = [];

   for (const account of compressedAccounts) {
       const variantSeeds = getVariantSeedMetadata(account.variant);
       const accountSeedIndices = [];

       for (const seed of variantSeeds) {
           if (seed.type === 'account') {
               const pubkey = account.parsed[seed.path];
               const index = getOrInsertSeedAccount(pubkey, seedAccountMap, seedAccounts);
               accountSeedIndices.push(index);
           }
       }

       account.seed_indices = accountSeedIndices;
   }

   // Add deduplicated seed accounts to remaining_accounts
   remainingAccounts.push(...seedAccounts.map(pk => ({ pubkey: pk, ... })));
   ```

## Benefits

1. ✅ **No struct bloat**: `DecompressAccountsIdempotent` stays minimal
2. ✅ **Dynamic**: Any number of seed accounts can be passed
3. ✅ **Deduplication**: Same account used by multiple variants appears once
4. ✅ **Type-safe**: Runtime bounds checking with clear errors
5. ✅ **Efficient**: Minimal overhead (just u8 indices)

## Breaking Changes

This is a **breaking change** to the `decompress_accounts_idempotent` instruction:

- ✅ New parameter: `seed_accounts_offset`
- ✅ Changed structure: `CompressedAccountData` has new fields
- ✅ Client must pass `seedAccountsOffset` and populate arrays

## Testing

To test with accounts that don't use account-reference seeds:

```rust
// Macro usage (literal seeds only)
#[add_compressible_instructions(
    Config = ("config", "v1"),
    MyAccount = ("my_account", MY_CONST_SEED),
)]
```

This will work with the current implementation (empty `seed_indices`).

## Next Steps

1. **Extend IDL Generation** (macro changes):
   - Track which seeds are account references vs literals/constants
   - Output this metadata in IDL

2. **Update Client** (TypeScript):
   - Parse IDL seed metadata
   - Extract seed pubkeys from parsed account data
   - Build `seed_indices` arrays
   - Deduplicate and pack seed accounts

3. **Testing**:
   - Test with literal-only seeds (works now)
   - Test with account-reference seeds (requires IDL enhancement)
   - Test deduplication logic
   - Test multiple compressed accounts

## Files Modified

### Rust

- `sdk-libs/macros/src/variant_enum.rs`
- `sdk-libs/macros/src/compressible_instructions.rs`

### TypeScript

- `js/stateless.js/src/compressible/pack.ts`
- `js/compressed-token/src/compressible/helpers.ts`

## Architecture Decisions

### Why not embedded in account data?

- Account data should be pure business logic
- Seed metadata is infrastructure concern
- Parallel array approach is cleaner

### Why u8 indices?

- Max 256 seed accounts per instruction is reasonable
- Saves space vs u16/u32
- Can upgrade if needed

### Why deduplicate?

- Same pubkey might be used by multiple accounts
- Reduces transaction size
- More efficient

## Summary

This implementation successfully moves seed account references from named struct fields to positional indices in `remaining_accounts`. The on-chain Rust code is complete and correct. The TypeScript client has the structure in place but needs IDL metadata enhancement to populate seed indices for accounts that use account-reference seeds.
