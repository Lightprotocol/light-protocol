# Unified Decompress Specification v3 (Final)

## Implementation Status

| Component                        | Status | Notes                                                              |
| -------------------------------- | ------ | ------------------------------------------------------------------ |
| `LightAta` type                  | DONE   | `sdk-libs/sdk/src/compressible/standard_types.rs`                  |
| `LightMint` type                 | DONE   | `sdk-libs/sdk/src/compressible/standard_types.rs`                  |
| Error variants                   | DONE   | `AtMostOneMintAllowed`, `MintAndTokensForbidden` in `sdk/error.rs` |
| `CompressedAccountVariant` enum  | DONE   | Includes `LightAta`, `LightMint` variants in `variant_enum.rs`     |
| `HasTokenVariant` detection      | DONE   | Detects both standard and legacy types                             |
| Runtime validation               | DONE   | Constraint checks in `decompress_runtime.rs`                       |
| Trait extension                  | DONE   | `collect_all_accounts`, `process_light_atas`, `process_light_mints`|
| `StandardCompressedVariant` trait| DONE   | `sdk/compressible/mod.rs` + macro impl in `variant_enum.rs`        |
| Client `DecompressInput` enum    | DONE   | `compressible-client/src/lib.rs` with Ata/Mint/ProgramData         |
| Runtime processing for LightAta  | DONE   | `ctoken-sdk/src/compressible/decompress_runtime.rs`                |
| Runtime processing for LightMint | DONE   | `ctoken-sdk/src/compressible/decompress_runtime.rs`                |
| Client builder updates           | DONE   | Uses `StandardCompressedVariant` trait for type-safe packing       |
| Tests for idempotent ATA/Mint    | DONE   | `csdk-anchor-full-derived-test/tests/basic_test.rs`                |

---

## Executive Summary

Extend `decompress_accounts_idempotent` to handle **all four account types** using **standard SDK types** (`LightAta`, `LightMint`) for ATAs and Mints. Programs only declare their PDAs and program-owned CToken accounts (Vaults).

**Critical Constraint**: `compress_accounts_idempotent` does NOT support ATA/Mint compression - those are compressed by the forester invoking the ctoken program directly.

---

## 1. Account Type Taxonomy

| Type          | Declaration                                | SDK Type                     | Owner          | Signing                       | Limit |
| ------------- | ------------------------------------------ | ---------------------------- | -------------- | ----------------------------- | ----- |
| **cPDA**      | `#[compressible(Foo = (...))]`             | Program-generated            | Program        | Program PDA seeds             | Any # |
| **CToken**    | `#[compressible(Vault = (is_token, ...))]` | Program-generated            | Program        | Program PDA seeds (authority) | Any # |
| **LightAta**  | NOT declared - always available            | `light_sdk::LightAta`        | User wallet    | Wallet signs tx               | Any # |
| **LightMint** | NOT declared - always available            | `light_sdk::LightMint`       | ctoken program | Authority signs               | Max 1 |

---

## 2. Constraints (Enforced at Runtime)

| Constraint                                 | Error                    | Rationale                                            |
| ------------------------------------------ | ------------------------ | ---------------------------------------------------- |
| Max 1 LightMint per instruction            | `AtMostOneMintAllowed`   | CMint decompression creates on-chain state           |
| LightMint + (LightAta OR CToken) forbidden | `MintAndTokensForbidden` | Both modify on-chain state, CPI context conflicts    |
| LightMint + cPDA allowed                   | -                        | PDAs use CPI context write, mint uses different path |
| Any combo of LightAta + CToken + cPDA      | -                        | All can share CPI context                            |

---

## 3. Architecture

### 3.1 Macro Declaration (`#[compressible(...)]`)

```rust
#[compressible(
    // PDAs - program-specific types
    UserRecord = ("user_record", ctx.authority, data.owner),
    GameSession = ("game_session", ctx.user, data.session_id.to_le_bytes()),

    // Program-owned CTokens (Vaults) - require authority seeds
    Vault = (is_token, "vault", ctx.cmint, authority = ("vault_authority")),

    // Data fields for seed params
    owner = Pubkey,
    session_id = u64,
)]
pub mod instructions { ... }
```

### 3.2 Generated `CompressedAccountVariant` Enum (variant_enum.rs)

```rust
pub enum CompressedAccountVariant {
    // Program-specific PDAs (from declaration)
    UserRecord(UserRecord),
    PackedUserRecord(PackedUserRecord),
    GameSession(GameSession),
    PackedGameSession(PackedGameSession),

    // Program-owned CTokens (Vaults) - if declared
    PackedCTokenData(PackedCTokenData<CTokenAccountVariant>),
    CTokenData(CTokenData<CTokenAccountVariant>),

    // ALWAYS included - standard SDK types (not from declaration)
    LightAta(light_sdk::compressible::LightAta),
    LightMint(light_sdk::compressible::LightMint),
}
```

### 3.3 Generated `CTokenAccountVariant` Enum (seed_providers.rs)

Only includes variants declared with `is_token`:

```rust
#[repr(u8)]
pub enum CTokenAccountVariant {
    Vault = 0,
}
```

Each variant implements `CTokenSeedProvider` (`get_seeds`, `get_authority_seeds`).

### 3.4 Runtime Flow (decompress_runtime.rs)

```
process_decompress_accounts_idempotent()
    |
    +-> check_account_types() -> (has_tokens, has_pdas, has_mints, mint_count, has_light_atas, has_light_mints)
    |
    +-> Validate constraints (max 1 mint, mint+tokens forbidden)
    |
    +-> ctx.collect_all_accounts() -> (pda_infos, token_accounts, light_atas, light_mints)
    |
    +-> if has_pdas: process PDAs via Light System CPI (write to CPI context if multi-type)
    |
    +-> if has_light_mints: ctx.process_light_mints() via DecompressCMint CPI
    |
    +-> if has_light_atas: ctx.process_light_atas() via Transfer2 CPI (consume CPI context)
    |
    +-> if has_tokens (CToken/Vaults): ctx.process_tokens() via Transfer2 CPI (consume CPI context)
```

---

## 4. Standard Types (light-sdk)

### 4.1 LightAta

Location: `sdk-libs/sdk/src/compressible/standard_types.rs`

```rust
/// Standard ATA for unified decompression.
/// Wallet must sign the transaction.
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct LightAta {
    /// Index into packed_accounts for wallet (must be signer)
    pub wallet_index: u8,
    /// Index into packed_accounts for mint
    pub mint_index: u8,
    /// Index into packed_accounts for derived ATA address
    pub ata_index: u8,
    /// Token amount to decompress
    pub amount: u64,
    /// Whether the token has a delegate
    pub has_delegate: bool,
    /// Delegate index (only valid if has_delegate is true)
    pub delegate_index: u8,
    /// Whether the token is frozen
    pub is_frozen: bool,
}
```

### 4.2 LightMint

```rust
/// Standard CMint for unified decompression.
/// The mint authority must sign (or fee_payer if it's the authority).
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct LightMint {
    // === Account indices ===
    /// Index into packed_accounts for mint_seed pubkey
    pub mint_seed_index: u8,
    /// Index into packed_accounts for derived CMint PDA
    pub cmint_pda_index: u8,
    /// Whether the mint has a mint authority
    pub has_mint_authority: bool,
    /// Mint authority index (only valid if has_mint_authority is true)
    pub mint_authority_index: u8,
    /// Whether the mint has a freeze authority
    pub has_freeze_authority: bool,
    /// Freeze authority index (only valid if has_freeze_authority is true)
    pub freeze_authority_index: u8,

    // === Raw data (not indices) ===
    /// Compressed account address (Light protocol address hash)
    pub compressed_address: [u8; 32],
    /// Token decimals
    pub decimals: u8,
    /// Total supply
    pub supply: u64,
    /// Metadata version
    pub version: u8,
    /// Whether mint has been decompressed before
    pub cmint_decompressed: bool,
    /// Rent payment in epochs (must be >= 2)
    pub rent_payment: u8,
    /// Lamports for future write operations
    pub write_top_up: u32,
    /// Extensions data (if any) - serialized ExtensionInstructionData
    pub extensions: Option<Vec<u8>>,
}
```

### 4.3 StandardCompressedVariant Trait

Location: `sdk-libs/sdk/src/compressible/mod.rs`

This trait enables the client builder to create `LightAta` and `LightMint` variants without knowing the program-specific `CompressedAccountVariant` type at compile time.

```rust
/// Builds standard LightAta/LightMint variants for compressible enums.
/// Implemented by macro-generated `CompressedAccountVariant` types.
pub trait StandardCompressedVariant: Pack {
    fn pack_light_ata(light_ata: LightAta) -> Self::Packed;
    fn pack_light_mint(light_mint: LightMint) -> Self::Packed;
}
```

### 4.4 Macro-Generated Implementation (variant_enum.rs)

```rust
impl light_sdk::compressible::StandardCompressedVariant for CompressedAccountVariant {
    fn pack_light_ata(light_ata: light_sdk::compressible::LightAta) -> Self::Packed {
        CompressedAccountVariant::LightAta(light_ata)
    }

    fn pack_light_mint(light_mint: light_sdk::compressible::LightMint) -> Self::Packed {
        CompressedAccountVariant::LightMint(light_mint)
    }
}
```

---

## 5. Client-Side Implementation

**File**: `sdk-libs/compressible-client/src/lib.rs`

### 5.1 DecompressInput Enum

```rust
/// Input type for decompress_accounts_idempotent instruction builder.
/// Allows mixing PDAs with standard ATAs and Mints.
pub enum DecompressInput<T> {
    /// Program-specific PDA or CToken account
    ProgramData(CompressedAccount, T),
    /// Standard ATA from compressed token account
    Ata(CompressedTokenAccount),
    /// Standard CMint from compressed mint
    Mint {
        compressed_account: CompressedAccount,
        mint_seed_pubkey: Pubkey,
        rent_payment: u8,
        write_top_up: u32,
    },
}
```

### 5.2 Updated Builder Function

```rust
/// Builds decompress_accounts_idempotent instruction with unified inputs.
///
/// Supports mixing:
/// - Program PDAs via `DecompressInput::ProgramData`
/// - Standard ATAs via `DecompressInput::Ata`
/// - Standard CMints via `DecompressInput::Mint`
///
/// # Constraints (validated at client + runtime):
/// - At most 1 mint per instruction
/// - Mint + (ATA/CToken) combination is forbidden
/// - Mint + PDAs is allowed
/// - Any combination of ATAs, CTokens, and PDAs works
#[allow(clippy::too_many_arguments)]
pub fn decompress_accounts_unified<T>(
    program_id: &Pubkey,
    discriminator: &[u8],
    decompressed_account_addresses: &[Pubkey],
    inputs: Vec<DecompressInput<T>>,
    program_account_metas: &[AccountMeta],
    validity_proof_with_context: ValidityProofWithContext,
) -> Result<Instruction, Box<dyn std::error::Error>>
where
    T: Pack + StandardCompressedVariant + Clone + std::fmt::Debug,
{
    // ... validation, packing, instruction building
}
```

### 5.3 Client-Side Packing

The builder uses `StandardCompressedVariant` to pack `LightAta` and `LightMint`:

```rust
DecompressInput::Ata(compressed_token) => {
    // Derive indices for wallet, mint, ATA
    let wallet_index = remaining_accounts.insert_or_get_config(wallet_pubkey, true, false);
    let mint_index = remaining_accounts.insert_or_get_read_only(mint_pubkey);
    let ata_index = remaining_accounts.insert_or_get(ata_address);

    let light_ata = LightAta {
        wallet_index, mint_index, ata_index,
        amount: compressed_token.token.amount,
        has_delegate: compressed_token.token.delegate.is_some(),
        delegate_index,
        is_frozen: compressed_token.token.state == AccountState::Frozen,
    };

    // Use trait to pack into program's variant type
    let packed_data = T::pack_light_ata(light_ata);
    // ...
}

DecompressInput::Mint { compressed_account, mint_seed_pubkey, rent_payment, write_top_up } => {
    // Derive indices and parse mint data
    let mint_seed_index = remaining_accounts.insert_or_get_read_only(mint_seed_pubkey);
    let cmint_pda_index = remaining_accounts.insert_or_get(cmint_pda);

    let light_mint = LightMint {
        mint_seed_index, cmint_pda_index,
        compressed_address: mint_data.metadata.compressed_address,
        // ... other fields from mint_data
    };

    // Use trait to pack into program's variant type
    let packed_data = T::pack_light_mint(light_mint);
    // ...
}
```

---

## 6. Runtime Processing (ctoken-sdk)

**File**: `sdk-libs/ctoken-sdk/src/compressible/decompress_runtime.rs`

### 6.1 LightAta Processing

```rust
pub fn process_decompress_light_atas_runtime<'info, 'b>(
    fee_payer: &AccountInfo<'info>,
    ctoken_program: &AccountInfo<'info>,
    ctoken_rent_sponsor: &AccountInfo<'info>,
    ctoken_cpi_authority: &AccountInfo<'info>,
    ctoken_config: &AccountInfo<'info>,
    config: &AccountInfo<'info>,
    light_atas: Vec<(LightAta, CompressedAccountMetaNoLamportsNoAddress)>,
    proof: ValidityProof,
    cpi_accounts: &CpiAccounts<'b, 'info>,
    post_system_accounts: &[AccountInfo<'info>],
    has_prior_context: bool,
) -> Result<(), ProgramError> {
    // For each LightAta:
    // 1. Resolve indices to AccountInfo (wallet, mint, ata)
    // 2. Validate wallet is signer
    // 3. Derive ATA and verify against provided ata_index
    // 4. Build DecompressFullIndices
    // 5. Invoke decompress_full_ctoken_accounts_with_indices CPI
}
```

### 6.2 LightMint Processing

```rust
pub fn process_decompress_light_mints_runtime<'info, 'b, A>(
    accounts_for_config: &A,
    fee_payer: &AccountInfo<'info>,
    ctoken_program: &AccountInfo<'info>,
    ctoken_rent_sponsor: &AccountInfo<'info>,
    ctoken_cpi_authority: &AccountInfo<'info>,
    ctoken_config: &AccountInfo<'info>,
    config: &AccountInfo<'info>,
    light_mints: Vec<(LightMint, CompressedAccountMetaNoLamportsNoAddress)>,
    proof: ValidityProof,
    cpi_accounts: &CpiAccounts<'b, 'info>,
    has_prior_context: bool,
    has_subsequent: bool,
) -> Result<(), ProgramError>
where
    A: MintDecompressContext<'info>,
{
    // For each LightMint:
    // 1. Resolve indices to AccountInfo (mint_seed, cmint_pda, authorities)
    // 2. Derive CMint PDA and verify
    // 3. Build CompressedMintWithContext from LightMint fields
    // 4. Build CompressedMintInstructionData
    // 5. Invoke DecompressCMintCpi or DecompressCMintCpiWithContext
}
```

---

## 7. Client Usage Examples

### 7.1 Decompress ATA Only

```rust
use light_compressible_client::{compressible_instruction, DecompressInput};

let compressed_ata = rpc.get_compressed_token_accounts_by_owner(&wallet, None, None).await?;

let inputs: Vec<DecompressInput<CompressedAccountVariant>> = vec![
    DecompressInput::Ata(compressed_ata),
];

let ix = compressible_instruction::decompress_accounts_unified(
    &program_id,
    &compressible_instruction::DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR,
    &[ata_address],
    inputs,
    &program_account_metas,
    validity_proof,
)?;

// Wallet must sign!
rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer, &wallet]).await?;
```

### 7.2 Decompress Mint Only

```rust
let inputs: Vec<DecompressInput<CompressedAccountVariant>> = vec![
    DecompressInput::Mint {
        compressed_account: compressed_mint,
        mint_seed_pubkey: mint_signer_pda,
        rent_payment: 2,
        write_top_up: 5_000,
    },
];

let ix = compressible_instruction::decompress_accounts_unified(
    &program_id,
    &DECOMPRESS_DISCRIMINATOR,
    &[cmint_pda],
    inputs,
    &program_account_metas,
    validity_proof,
)?;
```

### 7.3 Mixed PDA + ATA

```rust
let inputs: Vec<DecompressInput<CompressedAccountVariant>> = vec![
    DecompressInput::ProgramData(user_record_compressed, user_record_data),
    DecompressInput::Ata(user_ata_compressed),
];

let ix = compressible_instruction::decompress_accounts_unified(
    &program_id,
    &DECOMPRESS_DISCRIMINATOR,
    &[user_record_pda, user_ata_address],
    inputs,
    &program_account_metas,
    validity_proof,
)?;
```

### 7.4 Forbidden: Mint + ATA (Will Fail)

```rust
// THIS WILL FAIL - mint + tokens forbidden
let inputs: Vec<DecompressInput<CompressedAccountVariant>> = vec![
    DecompressInput::Ata(user_ata_compressed),
    DecompressInput::Mint { /* ... */ },
];
// Error: MintAndTokensForbidden
```

---

## 8. Processing Order (Fixed)

```
1. PDAs      -> Light System CPI (write to CPI context if 2+ types)
2. LightMint -> DecompressCMint CPI (write to CPI context if tokens follow)
3. LightAta + CToken -> Transfer2 CPI (consume CPI context if prior writes)
```

Tokens (LightAta + CToken) are ALWAYS processed last because they consume the CPI context.

---

## 9. Files Modified

### 9.1 Standard Types (light-sdk)

| File                                      | Change                                       |
| ----------------------------------------- | -------------------------------------------- |
| `sdk/src/compressible/standard_types.rs`  | `LightAta`, `LightMint` types                |
| `sdk/src/compressible/mod.rs`             | `StandardCompressedVariant` trait + exports  |

### 9.2 Macros (sdk-libs/macros)

| File                                 | Change                                                |
| ------------------------------------ | ----------------------------------------------------- |
| `src/compressible/variant_enum.rs`   | Always add `LightAta`, `LightMint` variants           |
| `src/compressible/variant_enum.rs`   | Implement `StandardCompressedVariant` trait           |
| `src/compressible/variant_enum.rs`   | Update trait impls for new variants                   |

### 9.3 SDK Runtime (sdk-libs/sdk)

| File                                     | Change                               |
| ---------------------------------------- | ------------------------------------ |
| `src/compressible/decompress_runtime.rs` | Detect LightAta/LightMint types      |
| `src/compressible/decompress_runtime.rs` | Add constraint validation            |
| `src/error.rs`                           | `AtMostOneMintAllowed`, `MintAndTokensForbidden` |

### 9.4 CToken SDK Runtime (sdk-libs/ctoken-sdk)

| File                                     | Change                                    |
| ---------------------------------------- | ----------------------------------------- |
| `src/compressible/decompress_runtime.rs` | `process_decompress_light_atas_runtime`   |
| `src/compressible/decompress_runtime.rs` | `process_decompress_light_mints_runtime`  |

### 9.5 Client (sdk-libs/compressible-client)

| File         | Change                                                      |
| ------------ | ----------------------------------------------------------- |
| `src/lib.rs` | `DecompressInput` enum with `Ata`/`Mint`/`ProgramData`      |
| `src/lib.rs` | Updated `decompress_accounts_unified` with trait bounds     |

---

## 10. NOT In Scope

- `compress_accounts_idempotent` does NOT support ATA/Mint compression
- ATAs and Mints are compressed by the forester invoking ctoken program directly
- No changes needed to compression flow

---

## 11. Visual Flow Diagram

```
                    +------------------------+
                    | Client builds inputs   |
                    | DecompressInput enum   |
                    +------------------------+
                              |
                    +---------v---------+
                    | validate_inputs   |
                    | - max 1 mint      |
                    | - mint+tokens !=  |
                    +-------------------+
                              |
                    +---------v---------+
                    | pack_inputs via   |
                    | StandardCompressed|
                    | Variant trait     |
                    +-------------------+
                              |
                    +---------v---------+
                    | TX to program     |
                    +-------------------+
                              |
       +----------------------+----------------------+
       |                                             |
+------v------+                               +------v------+
| On-chain    |                               | collect_all |
| validation  |                               | _accounts   |
| - same checks|                              +-------------+
+-------------+                                      |
                          +-------------+------------+-------------+
                          |             |            |             |
                    +-----v-----+ +-----v-----+ +----v----+ +------v------+
                    | PDAs      | | LightMint | | CToken  | | LightAta    |
                    +-----------+ +-----------+ +---------+ +-------------+
                          |             |            |             |
                    +-----v-----+ +-----v-----+      +------v------+
                    | Light Sys | | Decompress|      | Transfer2   |
                    | CPI write | | CMint CPI |      | CPI consume |
                    +-----------+ +-----------+      +-------------+
                          |             |                   |
                          +------+------+-------------------+
                                 |
                          +------v------+
                          | Done        |
                          +-------------+
```

---

## 12. Summary

| Old Design                              | New Design                                   |
| --------------------------------------- | -------------------------------------------- |
| Declare ATAs with `is_ata` attribute    | Use `LightAta` (standard, always available)  |
| Declare CMint variants manually         | Use `LightMint` (standard, always available) |
| Client manually packs ATAs              | Client uses `DecompressInput::Ata`           |
| Client manually packs Mints             | Client uses `DecompressInput::Mint`          |
| No constraint validation                | Client + on-chain validation                 |

**Benefits:**

1. **Simpler macro declarations** - No ATA/Mint variants to declare
2. **Standard client API** - `DecompressInput` enum handles all types uniformly
3. **Explicit constraints** - Clear errors for forbidden combinations
4. **Type safety** - Can't confuse ATAs with Vaults
5. **Trait-based packing** - `StandardCompressedVariant` enables generic client code

**Constraints enforced:**

1. `AtMostOneMintAllowed` - Max 1 LightMint per instruction
2. `MintAndTokensForbidden` - LightMint + tokens combination blocked

**Processing order (fixed):** PDAs -> LightMint -> CToken/LightAta
