# LightAccounts Derive Macro and Trait Derives

## 1. Overview

### 1.0 `#[light_account(...)]` Account Types

The `#[light_account(...)]` attribute supports four account types, each with its own namespace:

| Type | Namespace | Documentation | Description |
|------|-----------|---------------|-------------|
| PDA | (none) | [pda.md](pda.md) | Compressed PDAs with address registration |
| Mint | `mint::` | [mint.md](mint.md) | Compressed mints with optional metadata |
| Token | `token::` | [token.md](token.md) | PDA-owned token accounts (vaults) |
| Associated Token | `associated_token::` | [associated_token.md](associated_token.md) | User ATAs for compressed tokens |

### 1.1 Overview

The `#[derive(LightAccounts)]` macro and associated trait derives enable rent-free compressed accounts on Solana with minimal boilerplate. These macros generate code for:

- Pre-instruction compression setup (`LightPreInit` trait)
- Post-instruction cleanup (`LightFinalize` trait)
- Account data serialization and hashing
- Pubkey compression to u8 indices

### 1.1 Module Structure

```
sdk-libs/macros/src/light_pdas/
|-- mod.rs                    # Module exports
|-- shared_utils.rs           # Common utilities (constant detection, identifier extraction)
|-- light_account_keywords.rs # Keyword validation for #[light_account] parsing
|
|-- accounts/                 # #[derive(LightAccounts)] for Accounts structs
|   |-- mod.rs                # Module entry point
|   |-- derive.rs             # Orchestration layer
|   |-- builder.rs            # Code generation builder
|   |-- parse.rs              # Struct-level parsing and field classification
|   |-- light_account.rs      # Unified #[light_account] attribute parsing
|   |-- pda.rs                # PDA block code generation
|   |-- mint.rs               # Mint action CPI generation
|   |-- token.rs              # Token account and ATA CPI generation
|   +-- variant.rs            # Variant enum generation for light_program
|
|-- account/                  # Trait derive macros for data structs
|   |-- mod.rs                # Module entry point
|   |-- light_compressible.rs # LightAccount derive implementation
|   |-- seed_extraction.rs    # Anchor seed extraction from #[account(...)]
|   +-- utils.rs              # Shared utilities (field extraction, type checks)
|
+-- seeds/                    # Simplified seed extraction (3-category system)
    |-- mod.rs                # Module entry point
    |-- types.rs              # ClassifiedSeed, SeedSource enums
    |-- extract.rs            # Seed extraction from Anchor attributes
    +-- classify.rs           # Seed classification logic
```

---

## 2. `#[derive(LightAccounts)]` Derive Macro

### 2.1 Purpose

Generates `LightPreInit` and `LightFinalize` trait implementations for Anchor Accounts structs. These traits enable automatic compression of PDA accounts, mint creation, and token account creation during instruction execution.

**Source**: `sdk-libs/macros/src/light_pdas/accounts/derive.rs`

### 2.2 Supported Attributes

#### `#[light_account(init)]` - Mark PDA Fields for Compression

Applied to `Account<'info, T>`, `Box<Account<'info, T>>`, or `AccountLoader<'info, T>` fields.

```rust
#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateParams)]
pub struct CreateAccounts<'info> {
    #[account(
        init,
        payer = fee_payer,
        space = 8 + UserRecord::INIT_SPACE,
        seeds = [b"user", params.owner.as_ref()],
        bump
    )]
    #[light_account(init)]  // Uses address_tree_info and output_tree from CreateAccountsProof
    pub user_record: Account<'info, UserRecord>,
}
```

**Note**: Tree info is automatically sourced from `CreateAccountsProof` in the instruction parameters. No additional arguments needed.

#### `#[light_account(init, zero_copy)]` - Zero-Copy PDA Fields

For `AccountLoader<'info, T>` fields using Pod (zero-copy) serialization:

```rust
#[account(
    init,
    payer = fee_payer,
    space = 8 + core::mem::size_of::<ZcRecord>(),
    seeds = [b"zc_record", params.owner.as_ref()],
    bump,
)]
#[light_account(init, zero_copy)]
pub zc_record: AccountLoader<'info, ZcRecord>,
```

**Requirements**:
- The `zero_copy` keyword is required for `AccountLoader` fields
- `AccountLoader` uses Pod serialization which is incompatible with Borsh decompression
- The data type must implement `bytemuck::Pod` and `bytemuck::Zeroable`

### 2.3 Namespace Syntax for `#[light_account]`

The `#[light_account]` attribute uses Anchor-style namespace prefixes to specify parameters for different account types.

#### Token Account Parameters (`token::`)

```rust
#[light_account(init, token,
    token::authority = [VAULT_SEED, self.offer.key()],  // PDA owner seeds (required)
    token::mint = token_mint_a,                          // Mint account field (required for init)
    token::owner = authority,                            // Owner field (required for init)
    token::bump = params.vault_bump                      // Optional: explicit bump
)]
pub vault: UncheckedAccount<'info>,
```

| Parameter | Description | Required |
|-----------|-------------|----------|
| `token::authority` | PDA seeds for the token account owner (array expression) | Yes |
| `token::mint` | Field reference for the token mint | Yes (init only) |
| `token::owner` | Field reference for the PDA owner | Yes (init only) |
| `token::bump` | Explicit bump seed (auto-derived if omitted) | No |

#### Mint Parameters (`mint::`)

```rust
#[light_account(init, mint,
    mint::signer = mint_signer,                           // AccountInfo that seeds the mint PDA (required)
    mint::authority = authority,                          // Mint authority field (required)
    mint::decimals = params.decimals,                     // Token decimals (required)
    mint::seeds = &[MINT_SIGNER_SEED, self.authority.key().as_ref()],  // PDA signer seeds (required)
    mint::bump = params.mint_signer_bump,                 // Optional: explicit bump
    mint::freeze_authority = freeze_auth,                 // Optional: freeze authority field
    mint::authority_seeds = &[b"auth", &[auth_bump]],     // Optional: PDA seeds if authority is a PDA
    mint::authority_bump = params.auth_bump,              // Optional: bump for authority_seeds
    mint::rent_payment = 16,                              // Optional: rent payment epochs (default: 16)
    mint::write_top_up = 766,                             // Optional: write top-up lamports (default: 766)
    mint::name = params.name.clone(),                     // Optional: TokenMetadata name
    mint::symbol = params.symbol.clone(),                 // Optional: TokenMetadata symbol
    mint::uri = params.uri.clone(),                       // Optional: TokenMetadata URI
    mint::update_authority = update_auth,                 // Optional: metadata update authority
    mint::additional_metadata = params.extra_metadata     // Optional: additional metadata
)]
pub cmint: UncheckedAccount<'info>,
```

| Parameter | Description | Required |
|-----------|-------------|----------|
| `mint::signer` | AccountInfo that seeds the mint PDA | Yes |
| `mint::authority` | Mint authority field reference | Yes |
| `mint::decimals` | Token decimals (expression) | Yes |
| `mint::seeds` | PDA signer seeds for mint_signer (without bump) | Yes |
| `mint::bump` | Explicit bump for mint_seeds (auto-derived if omitted) | No |
| `mint::freeze_authority` | Optional freeze authority field | No |
| `mint::authority_seeds` | PDA seeds if authority is a PDA (without bump) | No |
| `mint::authority_bump` | Explicit bump for authority_seeds | No |
| `mint::rent_payment` | Rent payment epochs (default: 16) | No |
| `mint::write_top_up` | Write top-up lamports (default: 766) | No |
| `mint::name` | TokenMetadata name | No* |
| `mint::symbol` | TokenMetadata symbol | No* |
| `mint::uri` | TokenMetadata URI | No* |
| `mint::update_authority` | Metadata update authority field | No |
| `mint::additional_metadata` | Additional metadata key-value pairs | No |

*Note: `name`, `symbol`, and `uri` must all be specified together or none at all.

#### Associated Token Account Parameters (`associated_token::`)

```rust
#[light_account(init, associated_token,
    associated_token::authority = owner,  // ATA owner field (required)
    associated_token::mint = mint,        // ATA mint field (required)
    associated_token::bump = params.ata_bump  // Optional: explicit bump
)]
pub user_ata: UncheckedAccount<'info>,
```

| Parameter | Description | Required |
|-----------|-------------|----------|
| `associated_token::authority` | ATA owner field reference | Yes |
| `associated_token::mint` | ATA mint field reference | Yes |
| `associated_token::bump` | Explicit bump (auto-derived if omitted) | No |

### 2.4 Mark-Only Mode

For token accounts and ATAs that are NOT being initialized (just marked for light_program discovery), omit `init`:

```rust
// Mark-only token - requires authority for seed derivation
#[light_account(token::authority = [VAULT_SEED, self.offer.key()])]
pub existing_vault: Account<'info, CToken>,

// Mark-only ATA - requires authority and mint for ATA derivation
#[light_account(associated_token::authority = owner, associated_token::mint = mint)]
pub existing_ata: Account<'info, CToken>,
```

Mark-only mode:
- Returns `None` from parsing (skipped by LightAccounts derive)
- Processed by `#[light_program]` for decompress/compress instruction generation
- Token: requires `token::authority`, forbids `token::mint` and `token::owner`
- ATA: requires both `associated_token::authority` and `associated_token::mint`

#### `#[instruction(...)]` - Specify Instruction Parameters (Required)

Must be present on the struct when using `#[light_account(init)]`.

```rust
#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateParams)]
pub struct CreateAccounts<'info> { ... }
```

### 2.5 Infrastructure Field Detection

Infrastructure fields are auto-detected by naming convention. No attribute required.

| Field Type | Accepted Names |
|------------|----------------|
| Fee Payer | `fee_payer`, `payer`, `creator` |
| Compression Config | `compression_config` |
| PDA Rent Sponsor | `pda_rent_sponsor`, `compression_rent_sponsor` |
| Light Token Config | `light_token_compressible_config` |
| Light Token Rent Sponsor | `light_token_rent_sponsor`, `rent_sponsor` |
| Light Token Program | `light_token_program` |
| Light Token CPI Authority | `light_token_cpi_authority` |

**Source**: `sdk-libs/macros/src/light_pdas/accounts/parse.rs`

### 2.6 Code Generation Flow

```
1. Parse
   |-- parse_light_accounts_struct() extracts:
   |   - Struct name and generics
   |   - #[light_account(init)] fields -> PdaField (with zero_copy flag)
   |   - #[light_account(init, mint, ...)] fields -> LightMintField
   |   - #[light_account(init, token, ...)] fields -> TokenAccountField
   |   - #[light_account(init, associated_token, ...)] fields -> AtaField
   |   - #[instruction] args
   |   - Infrastructure fields by naming convention
   |
2. Validate
   |-- Total fields <= 255 (u8 index limit)
   |-- #[instruction] required when #[light_account] present
   |-- AccountLoader requires zero_copy keyword
   |-- Non-AccountLoader forbids zero_copy keyword
   |
3. Generate pre_init Body
   |-- Token accounts + ATAs: generate in pre_init (before instruction logic)
   |-- PDAs + Mints: generate compression CPI code
   |   - Zero-copy PDAs use different serialization path
   |   - Borsh PDAs use standard compression
   |
4. Wrap in Trait Impls
   |-- LightPreInit<'info, ParamsType>
   +-- LightFinalize<'info, ParamsType>
```

**Source**: `sdk-libs/macros/src/light_pdas/accounts/derive.rs`

### 2.7 Generated Code Example

**Input**:

```rust
#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateParams)]
pub struct CreateAccounts<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    pub compression_config: Account<'info, CompressibleConfig>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + UserRecord::INIT_SPACE,
        seeds = [b"user", params.owner.as_ref()],
        bump
    )]
    #[light_account(init)]
    pub user_record: Account<'info, UserRecord>,
}
```

**Output** (simplified):

```rust
#[automatically_derived]
impl<'info> light_sdk::compressible::LightPreInit<'info, CreateParams> for CreateAccounts<'info> {
    fn light_pre_init(
        &mut self,
        _remaining: &[solana_account_info::AccountInfo<'info>],
        params: &CreateParams,
    ) -> std::result::Result<bool, light_sdk::error::LightSdkError> {
        use anchor_lang::ToAccountInfo;

        // Build CPI accounts
        let cpi_accounts = light_sdk::cpi::v2::CpiAccounts::new(
            &self.fee_payer,
            _remaining,
            crate::LIGHT_CPI_SIGNER,
        );

        // Load compression config
        let compression_config_data = light_sdk::compressible::CompressibleConfig::load_checked(
            &self.compression_config,
            &crate::ID
        )?;

        // Collect compressed infos
        let mut all_compressed_infos = Vec::with_capacity(1);

        // PDA 0: user_record
        let __account_info_0 = self.user_record.to_account_info();
        let __account_key_0 = __account_info_0.key.to_bytes();
        let __new_addr_params_0 = { /* NewAddressParamsAssignedPacked */ };
        let __address_0 = light_compressed_account::address::derive_address(/* ... */);
        let __account_data_0 = &mut *self.user_record;
        let __compressed_infos_0 = light_sdk::compressible::prepare_compressed_account_on_init::<UserRecord>(/* ... */)?;
        all_compressed_infos.push(__compressed_infos_0);

        // Execute Light System Program CPI
        use light_sdk::cpi::{InvokeLightSystemProgram, LightCpiInstruction};
        light_sdk::cpi::v2::LightSystemProgramCpi::new_cpi(
            crate::LIGHT_CPI_SIGNER,
            params.create_accounts_proof.proof.clone()
        )
            .with_new_addresses(&[__new_addr_params_0])
            .with_account_infos(&all_compressed_infos)
            .invoke(cpi_accounts)?;

        Ok(true)
    }
}

#[automatically_derived]
impl<'info> light_sdk::compressible::LightFinalize<'info, CreateParams> for CreateAccounts<'info> {
    fn light_finalize(
        &mut self,
        _remaining: &[solana_account_info::AccountInfo<'info>],
        params: &CreateParams,
        _has_pre_init: bool,
    ) -> std::result::Result<(), light_sdk::error::LightSdkError> {
        use anchor_lang::ToAccountInfo;
        Ok(())
    }
}
```

---

## 3. Program Variants

The `#[derive(LightAccounts)]` macro supports five program variants based on the types of light_account fields present:

| Variant | Description | Fields |
|---------|-------------|--------|
| **PDA-only** | Only PDA fields with `#[light_account(init)]` | PDAs |
| **Token-only** | Only token account fields | `token::` |
| **Mint-only** | Only mint fields | `mint::` |
| **ATA-only** | Only associated token account fields | `associated_token::` |
| **Mixed** | Combination of any above | Multiple types |

Each variant generates appropriate code for the specific account types present.

---

## 3.1 Direct Proof Argument Support

By default, the macro expects `CreateAccountsProof` to be nested inside a params struct:

```rust
#[instruction(params: CreateParams)]  // params.create_accounts_proof
```

You can also pass `CreateAccountsProof` directly as an instruction argument:

```rust
#[instruction(proof: CreateAccountsProof)]
```

When `CreateAccountsProof` is detected as a direct instruction argument, the generated code automatically uses the correct field access (e.g., `proof.address_tree_info` instead of `params.create_accounts_proof.address_tree_info`).

---

## 3.2 Infrastructure Requirements Summary

The macro auto-detects infrastructure fields by naming convention. No attribute required.

### For PDAs

| Field Type | Accepted Names |
|------------|----------------|
| Fee Payer | `fee_payer`, `payer`, `creator` |
| Compression Config | `compression_config` |
| PDA Rent Sponsor | `pda_rent_sponsor`, `compression_rent_sponsor` |

### For Mints, Tokens, ATAs

| Field Type | Accepted Names |
|------------|----------------|
| Fee Payer | `fee_payer`, `payer`, `creator` |
| Light Token Config | `light_token_compressible_config` |
| Light Token Rent Sponsor | `light_token_rent_sponsor`, `rent_sponsor` |
| Light Token Program | `light_token_program` |
| Light Token CPI Authority | `light_token_cpi_authority` |

---

## 3.3 Validation Rules Summary

The macro validates at compile time:

### PDA Fields
- `init` is required
- `zero_copy` is required for `AccountLoader` fields
- `zero_copy` is forbidden for non-`AccountLoader` fields
- No additional namespace parameters allowed (tree info auto-fetched)

### Token Fields
- `token::authority` is always required
- For init mode: `token::mint` and `token::owner` are required
- For mark-only mode: `token::mint` and `token::owner` are NOT allowed

### Associated Token Fields
- `associated_token::authority` and `associated_token::mint` are always required

### Mint Fields
- `mint::signer`, `mint::authority`, `mint::decimals`, `mint::seeds` are required
- TokenMetadata fields (`name`, `symbol`, `uri`) must all be specified together
- `update_authority` and `additional_metadata` require core metadata fields

### Namespace Validation
- Parameters must use the correct namespace for the account type
- Mixing namespaces (e.g., `token::authority` with `associated_token::mint`) causes a compile error
- Duplicate keys within the same attribute cause a compile error

---

## 4. Data Struct Derives (account/)

The `#[derive(LightAccount)]` macro generates all traits needed for compressible account data structs.

See **`../account/architecture.md`** for detailed documentation.

### Quick Reference

```
#[derive(LightAccounts)]              <- Accounts struct (this file)
    |
    +-- Generates LightPreInit + LightFinalize impls
    |
    +-- Uses traits from data struct derives:
        |
        +-- #[derive(LightAccount)]   <- Data struct (account/architecture.md)
            |
            +-- DataHasher + ToByteArray (SHA256 hashing)
            +-- LightDiscriminator (8-byte unique ID)
            +-- Pack + Unpack + Packed{Name} struct
            +-- compression_info accessors
```

### Usage

```rust
// Data struct - apply LightAccount
#[derive(LightAccount, LightDiscriminator, LightHasherSha)]
#[account]
pub struct UserRecord {
    pub compression_info: CompressionInfo,  // Required, first or last field
    pub owner: Pubkey,
    pub score: u64,
}

// Accounts struct - apply LightAccounts
#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateParams)]
pub struct Create<'info> {
    #[account(init, ...)]
    #[light_account(init)]
    pub user_record: Account<'info, UserRecord>,
}
```

---

## 5. Source Code Structure

```
sdk-libs/macros/src/light_pdas/
|
|-- mod.rs                  Module exports
|-- shared_utils.rs         Common utilities (MetaExpr, type helpers)
|-- light_account_keywords.rs  Keyword validation for #[light_account]
|
|-- accounts/               #[derive(LightAccounts)] for ACCOUNTS structs
|   |-- mod.rs              Entry point, exports derive_light_accounts()
|   |-- derive.rs           Orchestration: parse -> validate -> generate
|   |-- builder.rs          LightAccountsBuilder for code generation
|   |-- parse.rs            Struct-level parsing and field classification
|   |-- light_account.rs    #[light_account] attribute parsing
|   |-- pda.rs              PDA compression block generation
|   |-- mint.rs             Mint action CPI generation
|   |-- token.rs            Token account and ATA CPI generation
|   +-- variant.rs          Variant enum generation for light_program
|
+-- account/                #[derive(LightAccount)] for DATA structs
    |-- mod.rs              Entry point for trait derives
    |-- light_compressible.rs  LightAccount derive implementation
    |-- seed_extraction.rs  Anchor seed parsing
    +-- utils.rs            Shared utilities
```

---

## 6. Limitations

### Field Limits
- **Maximum 255 fields**: Total `#[light_account]` fields must be <= 255 (u8 index limit)
- **Single instruction param**: Only one `#[instruction(param: Type)]` is supported

### Type Restrictions
- `#[light_account(init)]` applies to `Account<'info, T>`, `Box<Account<'info, T>>`, or `AccountLoader<'info, T>` fields
- Nested `Box<Box<Account<...>>>` is not supported
- `AccountLoader` requires `zero_copy` keyword; `Account` forbids it

### Zero-Copy Constraints
- Zero-copy accounts use Pod serialization, incompatible with Borsh decompression
- Data types must implement `bytemuck::Pod` and `bytemuck::Zeroable`
- Zero-copy is for performance-critical accounts with fixed layouts

### No-op Fallback
When no `#[instruction]` attribute is present, the macro generates no-op implementations for backwards compatibility with non-compressible Accounts structs.

---

## 7. Related Documentation

### Account Type Documentation

- **`pda.md`** - Compressed PDA creation with `#[light_account(init)]`
- **`mint.md`** - Compressed mint creation with `#[light_account(init, mint::...)]`
- **`token.md`** - Token account creation with `#[light_account(init, token::...)]`
- **`associated_token.md`** - ATA creation with `#[light_account(init, associated_token::...)]`

### Other References

- **`../light_program/`** - Program-level `#[light_program]` attribute macro (architecture.md + codegen.md)
- **`../../README.md`** - Package overview
- **`sdk-libs/sdk/`** - Runtime SDK with `LightPreInit`, `LightFinalize` trait definitions
