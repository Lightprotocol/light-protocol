# Feature Comparison: Anchor vs Light Protocol RentFree

This document provides a comprehensive comparison between Anchor's account macros, Anchor SPL features, and Light Protocol's rentfree macro system.

## Account Initialization Methods

| Feature | Anchor | Anchor SPL | Light RentFree |
|---------|--------|------------|----------------|
| Create account | `init` | `init` | `init` (same) |
| Idempotent create | `init_if_needed` | `init_if_needed` | `init_if_needed` + compression check |
| Pre-allocated | `zero` | - | `zero` (same) |
| PDA creation | `seeds + bump + init` | `seeds + bump + init` | `seeds + bump + init` + address registration |
| Token account | - | `token::*` | `#[light_account(token)]` |
| Mint creation | - | `mint::*` | `#[light_account(init)]` |
| ATA creation | - | `associated_token::*` | Via `light_pre_init()` |

## Constraint Types Matrix

| Constraint | Anchor | Anchor SPL | Light RentFree | Notes |
|------------|--------|------------|----------------|-------|
| `mut` | Yes | Yes | Yes | Identical |
| `signer` | Yes | Yes | Yes | Identical |
| `close` | Yes | Yes | Yes + compression cleanup | Extended |
| `realloc` | Yes | Yes | Limited | Compression affects this |
| `has_one` | Yes | Yes | Yes | Identical |
| `owner` | Yes | Yes | Yes + compression owner | Extended |
| `address` | Yes | Yes | Yes | Identical |
| `executable` | Yes | Yes | Yes | Identical |
| `seeds + bump` | Yes | Yes | Yes + address derivation | Extended |
| `rent_exempt` | Yes | Yes | N/A | Rent-free by design |
| `constraint` | Yes | Yes | Yes | Identical |
| `token::mint` | - | Yes | Via `#[light_account(token)]` | Different syntax |
| `token::authority` | - | Yes | Via `#[light_account(token)]` | Different syntax |
| `mint::decimals` | - | Yes | Via `#[light_account(init)]` | Different syntax |
| `mint::authority` | - | Yes | Via `#[light_account(init)]` | Different syntax |
| `compression_info` | - | - | Yes | RentFree only |
| `compress_as` | - | - | Yes | RentFree only |

## Execution Lifecycle Phases

### Phase Comparison

| Phase | Anchor | Light RentFree |
|-------|--------|----------------|
| 1. Extract AccountInfo | `try_accounts()` | `try_accounts()` |
| 2. System CPI (create) | `try_accounts()` | `try_accounts()` |
| 3. Token/Mint CPI | `try_accounts()` | `light_pre_init()` |
| 4. Deserialize | `try_accounts()` | `try_accounts()` |
| 5. Address registration | - | `light_pre_init()` |
| 6. Instruction handler | After `try_accounts()` | After `light_pre_init()` |
| 7. Compression finalize | - | `light_finalize()` |
| 8. Exit (close, etc.) | `exit()` | `exit()` + compression |

### Visual Flow

```
ANCHOR:
================================================================================
try_accounts() ─────────────────────────────────────────────> handler() -> exit()
     │
     ├─ Extract AccountInfo
     ├─ System CPI (init)
     ├─ Token CPI (mint::*, token::*)
     └─ Deserialize
================================================================================

LIGHT RENTFREE:
================================================================================
try_accounts() ───> light_pre_init() ───> handler() ───> light_finalize() -> exit()
     │                    │                                    │
     ├─ Extract           ├─ Register address                  ├─ Serialize state
     ├─ System CPI        ├─ Compressed mint CPI               ├─ Create Merkle leaf
     └─ Deserialize       └─ Compression setup                 └─ Update tree
================================================================================
```

## Deserialization Behavior

| Scenario | Anchor | Light RentFree |
|----------|--------|----------------|
| New account (init) | Borsh deserialize after CPI | Borsh deserialize after CPI |
| Existing account | Borsh deserialize | Borsh deserialize |
| Compressed account | - | Merkle proof verify + deserialize |
| Mixed state | - | Check `compression_info.is_compressed` |
| Token account | SPL token layout | SPL token layout + compression_info |
| Mint account | SPL mint layout | SPL mint layout + compressed mint link |

## Equivalence Mapping

### Anchor -> RentFree Equivalents

| Anchor Pattern | RentFree Equivalent |
|----------------|---------------------|
| `Account<'info, T>` | `Account<'info, T>` (with RentFree derive) |
| `#[account(init)]` | `#[account(init)]` + compression hooks |
| `#[account(init, token::mint = m)]` | `#[light_account(token)]` + `#[light_account(init)]` |
| `Program<'info, Token>` | `Program<'info, CompressedToken>` |
| `Account<'info, Mint>` | `UncheckedAccount<'info>` + `#[light_account(init)]` |
| `Account<'info, TokenAccount>` | `Account<'info, T>` with `#[light_account(token)]` |
| `close = destination` | `close = destination` + compression cleanup |
| Manual token CPI | Auto-generated in `light_pre_init()` |

### Type Mapping

| Anchor SPL Type | Light Protocol Type |
|-----------------|---------------------|
| `Mint` | `UncheckedAccount` (during init with `#[light_account(init, mint::...)]`) |
| `TokenAccount` | `UncheckedAccount` with `#[light_account(token::...)]` |
| `Token` program | `CompressedToken` program |
| `TokenInterface` | Not yet supported |
| `InterfaceAccount` | Not yet supported |

## CPI Injection Patterns

### Anchor Token CPI
```rust
// Anchor: CPI happens in try_accounts() during init
#[account(
    init,
    payer = user,
    mint::decimals = 6,
    mint::authority = user
)]
pub mint: Account<'info, Mint>,
```

### Light Protocol Token CPI
```rust
// Light Protocol: CPI happens in light_pre_init() after try_accounts()
/// CHECK: Created in light_pre_init
#[account(mut)]
#[light_account(init, mint::decimals = 6, mint::authority = user)]
pub mint: UncheckedAccount<'info>,
```

### Why the Difference?

1. **Anchor**: Mint exists during `try_accounts()`, so can use typed `Account<'info, Mint>`
2. **Light Protocol**: Mint created AFTER `try_accounts()`, so must use `UncheckedAccount`

```
Anchor timeline:
[System create] -> [Token init_mint] -> [Deserialize as Mint] -> [Handler]
                                              ↑
                                        Typed access OK

Light Protocol timeline:
[System create] -> [Deserialize as Unchecked] -> [light_pre_init: create compressed mint] -> [Handler]
                          ↑                              ↑
                    No type yet                    Compression happens here
```

## Data Struct Requirements

| Requirement | Anchor | Light Protocol |
|-------------|--------|----------------|
| Discriminator | Auto (8 bytes) | Auto (8 bytes) via LightDiscriminator |
| Borsh derive | Required | Required |
| Space calculation | Manual or `InitSpace` | Manual or `InitSpace` |
| CompressionInfo field | - | Required (non-Option, first or last) |
| Hash attributes | - | No `#[hash]` needed (SHA256 serializes full struct) |
| Pack/Unpack traits | - | Generated by `#[derive(LightAccount)]` |

### Anchor Data Struct
```rust
#[account]
pub struct MyData {
    pub owner: Pubkey,
    pub value: u64,
}
// Space: 8 (discriminator) + 32 + 8 = 48
```

### Light Protocol Data Struct
```rust
#[derive(Default, Debug, InitSpace, LightAccount, LightDiscriminator, LightHasherSha)]
#[account]
pub struct MyData {
    pub compression_info: CompressionInfo,  // Non-Option, first or last field
    pub owner: Pubkey,
    pub value: u64,
}
// Space: 8 (discriminator) + CompressionInfo + 32 + 8
```

## Limitations Comparison

| Limitation | Anchor | Anchor SPL | Light Protocol |
|------------|--------|------------|----------------|
| Rent cost | Full rent | Full rent | Zero rent (compressed) |
| Account size limit | 10MB | 10MB | 800 bytes (enforced by LightAccount) |
| Realloc support | Full | Full | Limited (compression boundary) |
| Interface accounts | Full | Full | Limited |
| Token-2022 | Full | Full | Partial |
| Cross-program composability | Full | Full | Requires proof |
| Immediate reads | Yes | Yes | Requires decompression or proof |
| Atomic updates | Yes | Yes | Yes (within tx) |
| Program complexity | Low | Low | Higher (proofs, trees) |
| Client complexity | Low | Low | Higher (proof generation) |

## Migration Path

### From Anchor to Light Protocol

1. **Add derives**: Add `LightAccount`, `LightDiscriminator`, `LightHasherSha`, `InitSpace`
2. **Add compression_info**: Add non-Option field as first or last field
3. **Update Accounts derives**: Add `LightAccounts` to `#[derive(Accounts)]`
4. **Add field attributes**: Add `#[light_account(init)]` to compressed PDA fields
5. **Update program attribute**: Add `#[light_program]` (optional, for auto-discovery)
6. **Add Light accounts**: Include protocol programs in accounts struct
7. **Update token handling**: Convert `mint::*` to `#[light_account(init, mint::...)]`

### Minimal Changes Example

**Before (Anchor)**:
```rust
#[account]
pub struct Counter {
    pub count: u64,
}
```

**After (Light Protocol)**:
```rust
#[derive(Default, Debug, InitSpace, LightAccount, LightDiscriminator, LightHasherSha)]
#[account]
pub struct Counter {
    pub compression_info: CompressionInfo,  // Non-Option, first or last field
    pub count: u64,
}
```

## When to Use Each

| Use Case | Recommended |
|----------|-------------|
| Standard Solana accounts | Anchor |
| SPL tokens (small scale) | Anchor SPL |
| High-volume token distribution | Light RentFree |
| Gaming (many player states) | Light RentFree |
| DeFi (composability critical) | Anchor + selective RentFree |
| NFT collections (large) | Light RentFree |
| DAO governance | Anchor (composability) |
| Airdrop campaigns | Light RentFree (cost) |
| Real-time trading | Anchor (speed) |
| Data archival | Light RentFree (cost) |

## Summary

| Aspect | Anchor | Light Protocol |
|--------|--------|----------------|
| **Primary benefit** | Simplicity, composability | Zero rent, scalability |
| **Learning curve** | Lower | Higher |
| **Runtime cost** | Rent + compute | Proof compute + no rent |
| **Best for** | General Solana dev | High-scale applications |
| **Ecosystem maturity** | Very mature | Growing |
| **Token support** | Full SPL + Token-2022 | Growing (SPL focus) |
| **Account size** | Up to 10MB | 800 bytes (enforced) |
