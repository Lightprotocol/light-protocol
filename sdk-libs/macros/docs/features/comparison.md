# Feature Comparison: Anchor vs Light Protocol RentFree

This document provides a comprehensive comparison between Anchor's account macros, Anchor SPL features, and Light Protocol's rentfree macro system.

## Account Initialization Methods

| Feature | Anchor | Anchor SPL | Light RentFree |
|---------|--------|------------|----------------|
| Create account | `init` | `init` | `init` (same) |
| Idempotent create | `init_if_needed` | `init_if_needed` | `init_if_needed` + compression check |
| Pre-allocated | `zero` | - | `zero` (same) |
| PDA creation | `seeds + bump + init` | `seeds + bump + init` | `seeds + bump + init` + address registration |
| Token account | - | `token::*` | `#[rentfree_token]` |
| Mint creation | - | `mint::*` | `#[light_mint]` |
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
| `token::mint` | - | Yes | Via `#[rentfree_token]` | Different syntax |
| `token::authority` | - | Yes | Via `#[rentfree_token]` | Different syntax |
| `mint::decimals` | - | Yes | Via `#[light_mint]` | Different syntax |
| `mint::authority` | - | Yes | Via `#[light_mint]` | Different syntax |
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
| `#[account(init, token::mint = m)]` | `#[rentfree_token]` + `#[light_mint]` |
| `Program<'info, Token>` | `Program<'info, CompressedToken>` |
| `Account<'info, Mint>` | `UncheckedAccount<'info>` + `#[light_mint]` |
| `Account<'info, TokenAccount>` | `Account<'info, T>` with `#[rentfree_token]` |
| `close = destination` | `close = destination` + compression cleanup |
| Manual token CPI | Auto-generated in `light_pre_init()` |

### Type Mapping

| Anchor SPL Type | Light RentFree Type |
|-----------------|---------------------|
| `Mint` | `UncheckedAccount` (during init) |
| `TokenAccount` | Custom struct with `#[rentfree_token]` |
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

### RentFree Token CPI
```rust
// RentFree: CPI happens in light_pre_init() after try_accounts()
/// CHECK: Created in light_pre_init
#[account(mut)]
#[light_account(init, mint,decimals = 6, authority = user)]
pub mint: UncheckedAccount<'info>,
```

### Why the Difference?

1. **Anchor**: Mint exists during `try_accounts()`, so can use typed `Account<'info, Mint>`
2. **RentFree**: Mint created AFTER `try_accounts()`, so must use `UncheckedAccount`

```
Anchor timeline:
[System create] -> [Token init_mint] -> [Deserialize as Mint] -> [Handler]
                                              ↑
                                        Typed access OK

RentFree timeline:
[System create] -> [Deserialize as Unchecked] -> [light_pre_init: create compressed mint] -> [Handler]
                          ↑                              ↑
                    No type yet                    Compression happens here
```

## Data Struct Requirements

| Requirement | Anchor | Light RentFree |
|-------------|--------|----------------|
| Discriminator | Auto (8 bytes) | Auto (8 bytes) |
| Borsh derive | Required | Required |
| Space calculation | Manual or `InitSpace` | Manual or `InitSpace` |
| CompressionInfo field | - | Required for compressible |
| compress_as attributes | - | Optional per field |
| Pack/Unpack traits | - | Generated by `#[derive(LightAccounts)]` |

### Anchor Data Struct
```rust
#[account]
pub struct MyData {
    pub owner: Pubkey,
    pub value: u64,
}
// Space: 8 (discriminator) + 32 + 8 = 48
```

### RentFree Data Struct
```rust
#[derive(RentFree, Compressible, HasCompressionInfo)]
#[light_account(init)]
pub struct MyData {
    #[compress_as(pubkey)]
    pub owner: Pubkey,
    pub value: u64,
    #[compression_info]
    pub compression_info: CompressionInfo,
}
// Space: 8 (discriminator) + 32 + 8 + CompressionInfo::SIZE
```

## Limitations Comparison

| Limitation | Anchor | Anchor SPL | Light RentFree |
|------------|--------|------------|----------------|
| Rent cost | Full rent | Full rent | Zero rent (compressed) |
| Account size limit | 10MB | 10MB | Effectively unlimited |
| Realloc support | Full | Full | Limited (compression boundary) |
| Interface accounts | Full | Full | Limited |
| Token-2022 | Full | Full | Partial |
| Cross-program composability | Full | Full | Requires proof |
| Immediate reads | Yes | Yes | Requires decompression or proof |
| Atomic updates | Yes | Yes | Yes (within tx) |
| Program complexity | Low | Low | Higher (proofs, trees) |
| Client complexity | Low | Low | Higher (proof generation) |

## Migration Path

### From Anchor to RentFree

1. **Add derives**: Add `RentFree`, `Compressible`, `HasCompressionInfo`
2. **Add compression_info**: Add field to data structs
3. **Add compress_as**: Annotate fields for hashing
4. **Update program attribute**: Add `#[rentfree_program]`
5. **Add Light accounts**: Include protocol programs in accounts struct
6. **Update token handling**: Convert `mint::*` to `#[light_mint]`

### Minimal Changes Example

**Before (Anchor)**:
```rust
#[account]
pub struct Counter {
    pub count: u64,
}
```

**After (LightAccounts)**:
```rust
#[derive(RentFree, Compressible, HasCompressionInfo)]
#[light_account(init)]
pub struct Counter {
    pub count: u64,
    #[compression_info]
    pub compression_info: CompressionInfo,
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

| Aspect | Anchor | Light RentFree |
|--------|--------|----------------|
| **Primary benefit** | Simplicity, composability | Zero rent, scalability |
| **Learning curve** | Lower | Higher |
| **Runtime cost** | Rent + compute | Proof compute + no rent |
| **Best for** | General Solana dev | High-scale applications |
| **Ecosystem maturity** | Very mature | Growing |
| **Token support** | Full SPL + Token-2022 | Growing (SPL focus) |
