# Macro Refactor: Feature Set Comparison

## Executive Summary

This document compares the **current implementation** of the Light Protocol macro system with the **proposed refactored design**. The refactor aims to:

1. **Simplify trait hierarchy** - Cleaner separation between data structs and account variants
2. **Move logic to SDK** - Macros generate types, SDK contains business logic
3. **Unify account handling** - Consistent treatment of PDAs, Tokens, ATAs, and Mints
4. **Add new SDK functions** - Generic lifecycle functions replace macro-generated code

### Key Changes at a Glance

| Aspect | Current | Refactor |
|--------|---------|----------|
| Data struct macro | `#[derive(LightCompressible)]` | `#[derive(LightAccount)]` |
| Accounts struct macro | `#[derive(LightAccounts)]` | `#[derive(LightAccounts)]` (enhanced) |
| Program macro | `#[light_program]` | `#[light_program]` (enhanced) |
| Trait count for data structs | 8+ traits | 1 trait (`LightAccount`) |
| Variant enum location | `#[light_program]` generates | `#[derive(LightAccounts)]` generates per-field |
| Token/ATA/Mint handling | Macro-generated code | SDK generic functions |
| Pre-init complexity | High (macro generates CPI code) | Low (calls SDK functions) |
| Discriminator | `#[derive(LightDiscriminator)]` | Same (Anchor-style, not breaking) |

---

## 1. Macro Names Comparison

| Purpose | Current Macro | Refactor Macro | Notes |
|---------|---------------|----------------|-------|
| Data struct traits | `#[derive(LightCompressible)]` | `#[derive(LightAccount)]` | Generates single trait instead of 8+ |
| Accounts struct traits | `#[derive(LightAccounts)]` | `#[derive(LightAccounts)]` | Enhanced with variant generation |
| Program-level codegen | `#[light_program]` | `#[light_program]` | Reduced responsibility |
| Hash implementation | `#[derive(LightHasherSha)]` | Included in `LightAccount` | Consolidated |
| Discriminator | `#[derive(LightDiscriminator)]` | Uses existing `#[derive(LightDiscriminator)]` | **Not breaking** |
| Compression traits | `#[derive(Compressible)]` | Included in `LightAccount` | Consolidated |
| Pack/Unpack | `#[derive(CompressiblePack)]` | Included in `LightAccount` | Consolidated |
| Pod support | `#[derive(PodCompressionInfoField)]` | `#[derive(LightAccount)]` with `zero_copy` | Unified |

---

## 2. Generated Items Comparison

### 2.1 LightCompressible vs LightAccount (Data Struct Macro)

| Generated Item | Current (`LightCompressible`) | Refactor (`LightAccount`) |
|----------------|-------------------------------|---------------------------|
| Packed struct | `Packed{Name}` | `Packed{Name}` |
| Hash trait | `DataHasher`, `ToByteArray` | `LightAccount::hash()` |
| Discriminator | `LightDiscriminator` | `LightAccount::DISCRIMINATOR` (uses existing derive) |
| Compression info accessors | `HasCompressionInfo` | `LightAccount::compression_info()` |
| Compress representation | `CompressAs` | Removed (hash includes all) |
| Size calculation | `Size` | `LightAccount::size()` |
| Init space | `CompressedInitSpace` | `LightAccount::INIT_SPACE` |
| Pack implementation | `Pack` trait | `LightAccount::pack()` |
| Unpack implementation | `Unpack` trait | `LightAccount::unpack()` |

**Current generates 8+ traits:**
```rust
impl DataHasher for MyStruct { ... }
impl ToByteArray for MyStruct { ... }
impl LightDiscriminator for MyStruct { ... }
impl HasCompressionInfo for MyStruct { ... }
impl CompressAs for MyStruct { ... }
impl Size for MyStruct { ... }
impl CompressedInitSpace for MyStruct { ... }
impl Pack for MyStruct { ... }
impl Unpack for MyStruct { ... }
impl Pack for PackedMyStruct { ... }
impl Unpack for PackedMyStruct { ... }
```

**Refactor generates 1 trait:**
```rust
pub struct PackedMyStruct { ... }
impl LightAccount for MyStruct {
    type Packed = PackedMyStruct;
    const DISCRIMINATOR: [u8; 8] = [...];
    const INIT_SPACE: usize = ...;
    fn hash<H: Hasher>(&self) -> Result<[u8; 32], HasherError>;
    fn compression_info(&self) -> Option<&CompressionInfo>;
    fn compression_info_mut(&mut self) -> Option<&mut CompressionInfo>;
    fn clear_compression_info(&mut self);
    fn size(&self) -> usize;
    fn pack(&self, accounts: &mut PackedAccounts) -> Result<Self::Packed, ProgramError>;
    fn unpack(packed: &Self::Packed, accounts: &[AccountInfo]) -> Result<Self, ProgramError>;
}
```

### 2.2 LightAccounts (Accounts Struct Macro)

| Generated Item | Current | Refactor |
|----------------|---------|----------|
| `LightPreInit` trait impl | Yes (complex CPI code) | Yes (calls SDK functions) |
| `LightFinalize` trait impl | Yes (no-op) | Yes (no-op) |
| Seeds structs | No (in `#[light_program]`) | Yes (`{Field}Seeds`, `Packed{Field}Seeds`) |
| Variant structs | No (in `#[light_program]`) | Yes (`{Field}Variant`, `Packed{Field}Variant`) |
| `LightAccountVariant` trait impl | No | Yes (per-variant) |
| `PackedLightAccountVariant` trait impl | No | Yes (per-variant) |
| Token/ATA/Mint seed structs | No (in `#[light_program]`) | Yes (seeds only, uses SDK generics) |

### 2.3 light_program (Program Macro)

| Generated Item | Current | Refactor |
|----------------|---------|----------|
| `LightAccountVariant` enum | Yes (all variants) | Yes (collects from `LightAccounts`) |
| `PackedLightAccountVariant` enum | No | Yes |
| `TokenAccountVariant` enum | Yes | No (SDK provides) |
| `{Type}Seeds` structs | Yes | No (moved to `LightAccounts`) |
| `{Type}CtxSeeds` structs | Yes | No (simplified) |
| `decompress_accounts_idempotent` | Yes | Yes (renamed `decompress_idempotent`) |
| `compress_accounts_idempotent` | Yes | Yes (renamed `compress_and_close`) |
| `initialize_compression_config` | Yes | Yes |
| `update_compression_config` | Yes | Yes |
| Client seed functions | Yes | Yes |
| Auto-wrapped handlers | Yes | Yes |
| Size validation | Yes | Yes |
| Error codes | Yes | Yes |
| `SeedParams` struct | Yes | No (removed) |

---

## 3. Trait System Comparison

### 3.1 Current Trait Hierarchy

```
Data Struct Level:
+-- DataHasher          (hash via byte array)
+-- ToByteArray         (convert to bytes)
+-- LightDiscriminator  (8-byte discriminator)
+-- HasCompressionInfo  (compression_info field accessors)
+-- CompressAs          (compressed representation)
+-- Size                (serialized size)
+-- CompressedInitSpace (COMPRESSED_INIT_SPACE const)
+-- Pack                (struct -> packed struct)
+-- Unpack              (packed struct -> struct)

Accounts Struct Level:
+-- LightPreInit<'info, P>  (pre-instruction setup)
+-- LightFinalize<'info, P> (post-instruction cleanup)

Program Level:
+-- Various generated traits for variant enums
+-- DecompressContext
+-- CompressContext
+-- TokenSeedProvider
+-- PdaSeedProvider
```

### 3.2 Refactor Trait Hierarchy

```
Data Struct Level:
+-- LightAccount               (all data struct functionality)
    +-- type Packed
    +-- DISCRIMINATOR, INIT_SPACE
    +-- hash(), compression_info(), size(), pack(), unpack()

Variant Level (per field):
+-- LightAccountVariant        (unpacked variant: seeds + data)
    +-- type Seeds, Data, Packed
    +-- SEED_COUNT
    +-- seeds(), data(), seed_refs(), derive_pda(), pack()

+-- PackedLightAccountVariant  (packed variant: bump + unpack)
    +-- type Unpacked
    +-- bump(), unpack()

Accounts Struct Level:
+-- LightPreInit<'info>        (simplified - calls SDK functions)
+-- LightFinalize<'info>       (no-op)

SDK Provided (not generated):
+-- TokenVariant<S>            (generic token variant)
+-- AtaVariant<S>              (generic ATA variant)
+-- MintVariant<S>             (generic mint variant)
+-- TokenSeeds, AtaSeeds, MintSeeds  (marker traits)
```

### 3.3 Trait Location Changes

| Trait | Current Location | Refactor Location |
|-------|------------------|-------------------|
| `DataHasher` | Generated by `LightHasherSha` | Method on `LightAccount` |
| `LightDiscriminator` | Generated by `LightDiscriminator` | Uses existing `LightDiscriminator` derive |
| `HasCompressionInfo` | Generated by `Compressible` | Methods on `LightAccount` |
| `CompressAs` | Generated by `Compressible` | **Removed** |
| `Size` | Generated by `Compressible` | Method on `LightAccount` |
| `Pack`/`Unpack` | Generated by `CompressiblePack` | Methods on `LightAccount` |
| `LightPreInit` | Generated with complex logic | Generated with SDK function calls |
| Variant traits | Generated by `#[light_program]` | Generated by `#[derive(LightAccounts)]` |

---

## 4. Account Types Handling

### 4.1 Overview

| Account Type | Current Handling | Refactor Handling |
|--------------|------------------|-------------------|
| **Pda** | Macro generates variant + seeds | Macro generates variant; SDK has logic |
| **PdaZeroCopy** | Special handling in macro | `#[light_account(init, zero_copy)]` |
| **Token** | Macro generates variant + CPI code | Seeds only; uses `TokenVariant<S>` from SDK |
| **Ata** | Macro generates variant + CPI code | Seeds only; uses `AtaVariant<S>` from SDK |
| **Mint** | Macro generates mint action CPI | Seeds only; uses `MintVariant<S>` from SDK |

### 4.2 PDA Handling

**Current:**
- `#[light_account(init)]` on Accounts struct
- `#[derive(LightCompressible)]` on data struct
- Seeds extracted from Anchor's `#[account(seeds = [...], bump)]`
- `LightAccountVariant` enum generated by `#[light_program]`
- Complex CPI code in generated `LightPreInit`

**Refactor:**
- `#[light_account(init)]` on Accounts struct
- `#[derive(LightAccount)]` on data struct
- Seeds extracted same way
- `{Field}Variant` struct generated by `#[derive(LightAccounts)]`
- Simple `light_pre_init_pda()` + `light_init_pdas()` SDK calls

### 4.3 Token Handling

**Current:**
```rust
#[light_account(init, token::authority = [...], token::mint = mint, token::owner = owner)]
pub vault: Account<'info, CToken>,
```
- Macro generates `CreateTokenAccountCpi` code
- Macro generates `TokenAccountVariant` enum variant
- Complex seed resolution in generated code

**Refactor:**
```rust
#[light_account(init, token, token::mint = mint, token::owner = owner, token::authority = [...])]
pub vault: UncheckedAccount<'info>,
```
- Macro generates only `VaultSeeds` struct implementing `TokenSeeds`
- SDK provides `TokenVariant<VaultSeeds>` generic
- `light_pre_init_token::<VaultVariant>()` SDK call in generated code

### 4.4 Mint Handling

**Current:**
```rust
#[light_account(init,
    mint::signer = mint_signer,
    mint::authority = authority,
    mint::decimals = 9,
    mint::seeds = &[b"mint"]
)]
pub mint: UncheckedAccount<'info>,
```
- Macro generates full `mint_action` CPI invocation
- Complex account resolution for config, rent sponsor, etc.
- Metadata extension handling in macro

**Refactor:**
- Same attribute syntax
- Macro generates only `MyMintSeeds` struct
- SDK provides `MintVariant<MyMintSeeds>` generic
- `light_pre_init_mint::<MyMintVariant>()` SDK call
- All CPI logic moved to SDK

---

## 5. Lifecycle Hooks Comparison

### 5.1 LightPreInit

**Current Implementation (complex, macro-generated):**
```rust
impl<'info> LightPreInit<'info, CreateParams> for CreateAccounts<'info> {
    fn light_pre_init(
        &mut self,
        remaining_accounts: &[AccountInfo<'info>],
        params: &CreateParams,
    ) -> Result<bool, LightSdkError> {
        // 100+ lines of generated code:
        // - CPI account building
        // - Address derivation
        // - Compressed account info preparation
        // - Light System Program CPI invocation
        // - Token account creation CPI
        // - Mint action CPI
        Ok(true)
    }
}
```

**Refactor Implementation (simple, SDK calls):**
```rust
impl<'info> LightPreInit<'info> for Create<'info> {
    fn light_pre_init(
        &mut self,
        proof: &CreateAccountsProof,
        remaining_accounts: &[AccountInfo<'info>],
        system_accounts_offset: u8,
    ) -> Result<()> {
        let cpi_accounts = CpiAccounts::new(remaining_accounts, system_accounts_offset);

        // 1. Collect PDA compressed infos
        let mut compressed_accounts = Vec::new();
        compressed_accounts.push(
            light_sdk::light_pre_init_pda::<UserRecordVariant>(&self.user_record)?
        );
        let num_pdas = compressed_accounts.len() as u8;

        // 2. Initialize all PDAs in single CPI
        light_sdk::light_init_pdas(&compressed_accounts, proof, remaining_accounts, &cpi_accounts)?;

        // 3. Initialize mints (if any)
        light_sdk::light_pre_init_mint::<MyMintVariant>(&self.my_mint, proof, remaining_accounts, &cpi_accounts, num_pdas)?;

        // 4. Initialize token accounts (if any)
        light_sdk::light_pre_init_token::<VaultVariant>(&self.vault, remaining_accounts, &cpi_accounts)?;

        Ok(())
    }
}
```

### 5.2 LightFinalize

**Current:** No-op (all work done in pre_init)

**Refactor:** No-op (same)

---

## 6. Instruction Generation Comparison

### 6.1 compress_accounts_idempotent vs compress_and_close

| Aspect | Current | Refactor |
|--------|---------|----------|
| Name | `compress_accounts_idempotent` | `compress_and_close` |
| Location | Generated in `#[light_program]` | Generated in `#[light_program]` |
| Logic | In generated code | SDK function call |
| Accounts struct | `CompressAccountsIdempotent<'info>` | `CompressAndClose<'info>` |

**Current:**
```rust
pub fn compress_accounts_idempotent(ctx: Context<CompressAccountsIdempotent>) -> Result<()> {
    // 50+ lines of generated code for CPI building
}
```

**Refactor:**
```rust
pub fn compress_and_close(ctx: Context<CompressAndClose>) -> Result<()> {
    light_sdk::compress_and_close::<ProgramAccountVariant>(
        ctx.accounts.data.as_ref(),
        ctx.remaining_accounts,
        CpiSigner::new(&crate::LIGHT_CPI_SIGNER),
    )
}
```

### 6.2 decompress_accounts_idempotent vs decompress_idempotent

| Aspect | Current | Refactor |
|--------|---------|----------|
| Name | `decompress_accounts_idempotent` | `decompress_idempotent` |
| Location | Generated in `#[light_program]` | Generated in `#[light_program]` |
| Logic | In generated code | SDK function call |
| Accounts struct | `DecompressAccountsIdempotent<'info>` | `DecompressIdempotent<'info>` |

**Current:**
```rust
pub fn decompress_accounts_idempotent(
    ctx: Context<DecompressAccountsIdempotent>,
    params: DecompressParams,
) -> Result<()> {
    // 100+ lines of generated code
}
```

**Refactor:**
```rust
pub fn decompress_idempotent(
    ctx: Context<DecompressIdempotent>,
    params: DecompressParams,
) -> Result<()> {
    light_sdk::decompress_idempotent::<DecompressParams, ProgramAccountVariant>(
        ctx.accounts.data.as_ref(),
        ctx.remaining_accounts,
        CpiSigner::new(&crate::LIGHT_CPI_SIGNER),
    )
}
```

---

## 7. SDK Functions (To Be Implemented)

The refactor moves heavy lifting from macro-generated code to SDK generic functions. These are marked for implementation:

| SDK Function | Purpose | Status |
|--------------|---------|--------|
| `light_pre_init_pda<V>()` | Returns `CompressedAccountInfo` for a PDA | To be implemented |
| `light_init_pdas()` | Batch init all PDAs with proof in single CPI | To be implemented |
| `light_pre_init_mint<V>()` | CPI to create compressed mint | To be implemented |
| `light_pre_init_token<V>()` | CPI to create compressed token account | To be implemented |
| `light_pre_init_ata<V>()` | CPI to create compressed ATA | To be implemented |
| `light_decompress_mints()` | CPI to decompress all mints | To be implemented |
| `compress_and_close<V>()` | Close PDA, insert into Merkle tree | To be implemented |
| `decompress_idempotent<P, V>()` | Recreate PDA from Merkle tree | To be implemented |
| `initialize_compression_config()` | Setup compression config PDA | To be implemented |
| `update_compression_config()` | Modify compression config | To be implemented |

Additional items to implement:
- Add `system_accounts_offset` to `CreateAccountsProof`
- SDK generic types: `TokenVariant<S>`, `AtaVariant<S>`, `MintVariant<S>`
- Marker traits: `TokenSeeds`, `AtaSeeds`, `MintSeeds`

---

## 8. Code Organization Comparison

### 8.1 Responsibility Distribution

| Responsibility | Current Location | Refactor Location |
|----------------|------------------|-------------------|
| Packed struct generation | `CompressiblePack` macro | `LightAccount` macro |
| Hash implementation | `LightHasherSha` macro | `LightAccount` macro |
| Discriminator generation | `LightDiscriminator` macro | Uses existing `LightDiscriminator` derive |
| Seeds struct generation | `#[light_program]` | `#[derive(LightAccounts)]` |
| Variant struct generation | `#[light_program]` | `#[derive(LightAccounts)]` |
| Program-wide enum | `#[light_program]` | `#[light_program]` |
| CPI account building | Macro-generated | SDK functions |
| Token/Mint/ATA creation | Macro-generated CPI | SDK generic functions |
| Compress/Decompress logic | Macro-generated | SDK functions |

### 8.2 Source Code Structure

**Current:**
```
src/
+-- hasher/           # LightHasher, LightHasherSha derives
+-- discriminator.rs  # LightDiscriminator derive
+-- light_pdas/
    +-- account/      # Compressible, CompressiblePack, LightCompressible
    +-- accounts/     # LightAccounts (pre_init, finalize)
    +-- program/      # light_program (enums, instructions, etc.)
    +-- seeds/        # Seed extraction utilities
```

**Refactor (conceptual):**
```
src/
+-- light_account.rs    # Single LightAccount derive
+-- light_accounts.rs   # LightAccounts with variant generation
+-- light_program.rs    # Simplified light_program

sdk/
+-- pre_init.rs         # light_pre_init_* functions
+-- compress.rs         # compress_and_close function
+-- decompress.rs       # decompress_idempotent function
+-- variants/           # TokenVariant, AtaVariant, MintVariant generics
```

---

## 9. Removed Features

| Feature | Current Status | Refactor Status | Reason |
|---------|----------------|-----------------|--------|
| `CompressAs` trait | Generated | **Removed** | Hash includes all fields |
| `SeedParams` struct | Generated | **Removed** | Simplified seed handling |
| `{Type}CtxSeeds` structs | Generated | **Removed** | Consolidated into variant |
| Complex CPI code in macro | Generated | **Removed** | Moved to SDK functions |
| `TokenAccountVariant` enum | Generated per-program | **Removed** | SDK provides generic |
| Multiple derive macros | 8+ macros | **1 macro** | Consolidated into `LightAccount` |
| Separate hashing macro | `LightHasherSha` | **Included** | Part of `LightAccount` |

---

## 10. New Features

| Feature | Refactor Addition | Benefit |
|---------|-------------------|---------|
| Single `LightAccount` trait | Unified data struct interface | Simpler, more maintainable |
| Per-field variant generation | `LightAccounts` generates variants | Better locality |
| SDK generic variants | `TokenVariant<S>`, `AtaVariant<S>`, `MintVariant<S>` | Reusable, testable |
| Batch PDA initialization | `light_init_pdas()` | Single CPI for all PDAs |
| Generic SDK functions | `light_pre_init_*()` family | Logic in SDK, not macro |
| `PdaZeroCopy` type | `#[light_account(init, zero_copy)]` | First-class zero-copy support |
| `system_accounts_offset` | In `CreateAccountsProof` | Cleaner remaining_accounts handling |
| Variant-level traits | `LightAccountVariant`, `PackedLightAccountVariant` | Clear abstraction boundary |

---

## 11. Implementation Details (Unchanged)

The following implementation details remain the same as the current implementation. See `implementation_details.md` for full documentation:

| Detail | Description |
|--------|-------------|
| **Discriminator** | Uses existing `#[derive(LightDiscriminator)]` (Anchor-style) |
| **Error handling** | Uses `anchor_lang::error::Error` and `ProgramError` |
| **Seed verification** | Seeds reconstructed and verified at decompress time |
| **Client seed helpers** | `seed_refs_with_bump()` on packed variants |
| **Size validation** | 800 byte limit, compile-time and runtime checks |
| **Nested field access** | Supports one level of nesting in seeds |
| **Zero-copy accounts** | Pod + BorshSerialize/Deserialize, borsh for pack/unpack |
| **Multiple PDAs** | Supported, initialized in declaration order |
| **`#[compress_as]`** | Field overrides with literals/constants |
| **`#[skip]`** | Excludes fields from hash/pack/size |
| **Constants in seeds** | Uppercase identifiers go directly to `seed_refs()` |

---

## 12. Summary Table

| Aspect | Current | Refactor | Impact |
|--------|---------|----------|--------|
| **Macro count** | 8+ derive macros | 2 derive + 1 attribute | Simpler API |
| **Trait count** | 10+ traits | 4 traits | Less boilerplate |
| **Code location** | Mostly in macros | Mostly in SDK | Better testing |
| **CPI generation** | Macro-generated | SDK functions | Maintainability |
| **Token handling** | Per-program variants | SDK generics | Reusability |
| **Variant generation** | At program level | At accounts level | Better locality |
| **Learning curve** | Steep (many macros) | Gentle (few macros) | Developer experience |
| **Compile times** | Higher (complex macros) | Lower (simpler macros) | Build performance |
| **Discriminator** | `LightDiscriminator` derive | Same derive (not breaking) | No migration needed |
| **Breaking changes** | N/A | Acceptable | Clean implementation |

---

## Appendix A: Trait Method Comparison

### LightAccount Trait (Refactor)

```rust
trait LightAccount: Sized + Clone + AnchorSerialize + AnchorDeserialize {
    type Packed: AnchorSerialize + AnchorDeserialize;
    const DISCRIMINATOR: [u8; 8];
    const INIT_SPACE: usize;

    fn hash<H: Hasher>(&self) -> Result<[u8; 32], HasherError>;
    fn compression_info(&self) -> Option<&CompressionInfo>;
    fn compression_info_mut(&mut self) -> Option<&mut CompressionInfo>;
    fn clear_compression_info(&mut self);
    fn size(&self) -> usize;
    fn pack(&self, accounts: &mut PackedAccounts) -> Result<Self::Packed, ProgramError>;
    fn unpack(packed: &Self::Packed, accounts: &[AccountInfo]) -> Result<Self, ProgramError>;
}
```

### Current Equivalent Traits

```rust
// DataHasher (from LightHasherSha)
trait DataHasher {
    fn hash<H: Hasher>(&self) -> Result<[u8; 32], HasherError>;
}

// LightDiscriminator
trait LightDiscriminator {
    const LIGHT_DISCRIMINATOR: [u8; 8];
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8];
}

// HasCompressionInfo (from Compressible)
trait HasCompressionInfo {
    fn compression_info(&self) -> Result<&CompressionInfo, ProgramError>;
    fn compression_info_mut(&mut self) -> Result<&mut CompressionInfo, ProgramError>;
    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo>;
    fn set_compression_info_none(&mut self) -> Result<(), ProgramError>;
}

// CompressAs (from Compressible) - REMOVED in refactor
trait CompressAs {
    type Output;
    fn compress_as(&self) -> Cow<'_, Self::Output>;
}

// Size (from Compressible)
trait Size {
    fn size(&self) -> Result<usize, ProgramError>;
}

// CompressedInitSpace (from Compressible)
trait CompressedInitSpace {
    const COMPRESSED_INIT_SPACE: usize;
}

// Pack (from CompressiblePack)
trait Pack {
    type Packed;
    fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Self::Packed;
}

// Unpack (from CompressiblePack)
trait Unpack {
    type Unpacked;
    fn unpack(&self, remaining_accounts: &[AccountInfo]) -> Result<Self::Unpacked, ProgramError>;
}
```

---

## Appendix B: File Locations

### Current Implementation

| Component | Path |
|-----------|------|
| LightCompressible | `src/light_pdas/account/light_compressible.rs` |
| Compressible traits | `src/light_pdas/account/traits.rs` |
| CompressiblePack | `src/light_pdas/account/pack_unpack.rs` |
| LightAccounts | `src/light_pdas/accounts/derive.rs` |
| light_program | `src/light_pdas/program/instructions.rs` |
| Hasher macros | `src/hasher/` |
| Discriminator | `src/discriminator.rs` |

### Refactor Specification

| Component | Path |
|-----------|------|
| Architecture overview | `docs/refactor/architecture.md` |
| LightAccount spec | `docs/refactor/light_account.md` |
| LightAccounts spec | `docs/refactor/light_accounts.md` |
| light_program spec | `docs/refactor/light_program.md` |
| Implementation details | `docs/refactor/implementation_details.md` |
