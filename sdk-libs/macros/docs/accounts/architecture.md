# LightAccounts Derive Macro and Trait Derives

## 1. Overview

### 1.0 `#[light_account(...)]` Account Types

The `#[light_account(...)]` attribute supports four account types, each with its own namespace:

| Type | Namespace | Documentation | Description |
|------|-----------|---------------|-------------|
| PDA | (none) | [pda.md](pda.md) | Light PDAs with address registration |
| Mint | `mint::` | [mint.md](mint.md) | Light mints with optional metadata |
| Token | `token::` | [token.md](token.md) | PDA-owned token accounts (vaults) |
| Associated Token | `associated_token::` | [associated_token.md](associated_token.md) | User ATAs for light tokens |

### 1.1 Overview

The `#[derive(LightAccounts)]` macro and associated trait derives enable rent-free light accounts on Solana with minimal boilerplate. These macros generate code for:

- Pre-instruction compression setup (`LightPreInit` trait)
- Post-instruction cleanup (`LightFinalize` trait)
- Account data serialization and hashing
- Pubkey compression to u8 indices

### 1.1 Module Structure

```
sdk-libs/macros/src/light_pdas/
|-- mod.rs                     # Module exports
|-- shared_utils.rs            # Common utilities (MetaExpr, type helpers, constant detection)
|-- light_account_keywords.rs  # Keyword validation for #[light_account] parsing
|
|-- accounts/                  # #[derive(LightAccounts)] for Accounts structs
|   |-- mod.rs                 # Module entry point
|   |-- derive.rs              # Orchestration layer
|   |-- builder.rs             # Code generation builder
|   |-- parse.rs               # Delegates to unified parsing module (type aliases)
|   |-- validation.rs          # Struct-level validation rules
|   |-- light_account.rs       # Unified #[light_account] attribute parsing
|   |-- pda.rs                 # PDA block code generation
|   |-- mint.rs                # Mint action CPI generation
|   |-- token.rs               # Token account and ATA CPI generation
|   +-- variant.rs             # Variant enum generation for light_program
|
|-- account/                   # Trait derive macros for data structs
|   |-- mod.rs                 # Module entry point
|   |-- derive.rs              # LightAccount derive implementation
|   |-- traits.rs              # Trait implementations (HasCompressionInfo, CompressAs, Compressible)
|   |-- validation.rs          # Shared validation utilities
|   +-- utils.rs               # Shared utilities (field extraction, type checks)
|
|-- parsing/                   # Unified parsing infrastructure
|   |-- mod.rs                 # Module exports
|   |-- accounts_struct.rs     # ParsedAccountsStruct for unified parsing
|   |-- crate_context.rs       # Crate-wide module parsing for struct discovery
|   |-- infra.rs               # Infrastructure field classification by naming convention
|   +-- instruction_arg.rs     # Instruction argument parsing from #[instruction(...)]
|
+-- seeds/                     # Seed extraction and classification
    |-- mod.rs                 # Module entry point
    |-- types.rs               # ClassifiedSeed, ExtractedSeedSpec type definitions
    |-- extract.rs             # Main extraction from Accounts structs
    |-- anchor_extraction.rs   # Extract seeds from #[account(seeds=[...])]
    |-- classification.rs      # Seed type classification (6 categories)
    |-- data_fields.rs         # Data field extraction from seeds
    +-- instruction_args.rs    # InstructionArgSet type definition
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
#[light_account(init,
    token::seeds = [VAULT_SEED, self.offer.key()],      // Token account PDA seeds (required, WITHOUT bump)
    token::owner_seeds = [VAULT_AUTH_SEED],             // Owner PDA seeds for decompression (required, WITHOUT bump)
    token::mint = token_mint_a,                         // Mint account field (required for init)
    token::owner = authority,                           // Owner field (required for init)
    token::bump = params.vault_bump                     // Optional: explicit bump for token::seeds
)]
pub vault: Account<'info, CToken>,
```

| Parameter | Description | Required |
|-----------|-------------|----------|
| `token::seeds` | Token account PDA seeds (WITHOUT bump) | Yes |
| `token::owner_seeds` | Owner PDA seeds for decompression (WITHOUT bump) | Yes |
| `token::mint` | Field reference for the token mint | Yes (init only) |
| `token::owner` | Field reference for the token owner/authority | Yes (init only) |
| `token::bump` | Explicit bump seed for token::seeds (auto-derived if omitted) | No |

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
)]
pub user_ata: UncheckedAccount<'info>,
```

| Parameter | Description | Required |
|-----------|-------------|----------|
| `associated_token::authority` | ATA owner field reference | Yes |
| `associated_token::mint` | ATA mint field reference | Yes |

### 2.4 Mark-Only Mode

For token accounts and ATAs that are NOT being initialized (just marked for light_program discovery), omit `init`:

```rust
// Mark-only token - requires seeds and owner_seeds for seed derivation
#[light_account(token::seeds = [VAULT_SEED, self.offer.key()], token::owner_seeds = [b"auth"])]
pub existing_vault: Account<'info, CToken>,

// Mark-only ATA - requires authority and mint for ATA derivation
#[light_account(associated_token::authority = owner, associated_token::mint = mint)]
pub existing_ata: Account<'info, CToken>,
```

Mark-only mode:
- Returns `None` from parsing (skipped by LightAccounts derive)
- Processed by `#[light_program]` for decompress/compress instruction generation
- Token: requires `token::seeds` and `token::owner_seeds`, forbids `token::mint` and `token::owner`
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
| Light Token Config | `light_token_config` |
| Light Token Rent Sponsor | `light_token_rent_sponsor`, `rent_sponsor` |
| Light Token Program | `light_token_program` |
| Light Token CPI Authority | `light_token_cpi_authority` |

**Source**: `sdk-libs/macros/src/light_pdas/accounts/parse.rs`

### 2.6 Execution Flow and Account Creation Timing

**Design principle**: ALL account creation happens in `LightPreInit` (before instruction handler execution) so that accounts are available for use during the instruction body.

#### When Accounts Are Created

| Account Type | Creation Phase | Builder/CPI Used |
|--------------|----------------|------------------|
| **PDAs** | `pre_init` | `LightSystemProgramCpi` or batched with mints |
| **Mints** | `pre_init` | `CreateMintsCpi` (batched, with optional PDA context) |
| **Token Accounts** | `pre_init` | `CreateTokenAccountCpi` with `rent_free()` |
| **ATAs** | `pre_init` | `CreateTokenAtaCpi` with `idempotent().rent_free()` |

#### Execution Timeline

```
1. Anchor deserializes accounts struct
2. light_pre_init() executes:
   a. Create token accounts (if any)
   b. Create ATAs (if any)
   c. Batch PDAs + Mints:
      - Write PDAs to CPI context
      - Create mints with decompress + offset
   OR PDAs only:
      - Register compressed addresses
   OR Mints only:
      - Create mints with decompress
3. Instruction handler executes (your code)
   - All accounts are now available
   - Can transfer tokens, mint, etc.
4. light_finalize() executes (currently no-op)
5. Anchor serializes account changes
```

### 2.7 Code Generation Flow

```
1. Parse (parse.rs, light_account.rs)
   |-- parse_light_accounts_struct() extracts:
   |   - Struct name and generics
   |   - #[light_account(init)] fields -> ParsedPdaField (with zero_copy flag)
   |   - #[light_account(init, mint::...)] fields -> LightMintField
   |   - #[light_account(init, token::...)] fields -> TokenAccountField
   |   - #[light_account(init, associated_token::...)] fields -> AtaField
   |   - #[instruction] args -> InstructionArg
   |   - Infrastructure fields by naming convention -> InfraFields
   |
2. Validate (validation.rs)
   |-- Total fields <= 255 (u8 index limit)
   |-- #[instruction] required when #[light_account(init)] present
   |-- AccountLoader requires zero_copy keyword
   |-- Non-AccountLoader forbids zero_copy keyword
   |-- Token/ATA fields require appropriate infrastructure fields
   |
3. Generate pre_init Body (builder.rs)
   |-- generate_pre_init_all() handles all combinations:
   |   - Token accounts: CreateTokenAccountCpi with PDA signing
   |   - ATAs: CreateTokenAtaCpi with idempotent mode
   |   - PDAs + Mints: Batched CPI with context offset
   |   - PDAs only: LightSystemProgramCpi
   |   - Mints only: CreateMintsCpi
   |   |
   |   PDA generation (pda.rs):
   |   - Zero-copy: load_init() + direct field access
   |   - Borsh: set_decompressed() + serialize
   |   |
   |   Mint generation (mint.rs):
   |   - Build SingleMintParams array
   |   - Invoke CreateMintsCpi with batching
   |   |
   |   Token/ATA generation (token.rs):
   |   - Build CPI structs with seed derivation
   |   - Call rent_free() builder methods
   |
4. Wrap in Trait Impls (builder.rs)
   |-- LightPreInit<'info, ParamsType>
   +-- LightFinalize<'info, ParamsType> (no-op)
```

**Source**: `sdk-libs/macros/src/light_pdas/accounts/derive.rs`

### 2.8 Generated Code Example

**Input**:

```rust
#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateParams)]
pub struct CreateAccounts<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    pub compression_config: Account<'info, CompressionConfig>,
    #[account(mut)]
    pub pda_rent_sponsor: Account<'info, RentSponsor>,

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
impl<'info> light_sdk::interface::LightPreInit<'info, CreateParams> for CreateAccounts<'info> {
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
        let compression_config_data = light_sdk::interface::LightConfig::load_checked(
            &self.compression_config,
            &crate::ID
        )?;

        // Prepare vectors for compression
        let mut all_new_address_params = Vec::with_capacity(1);
        let mut all_compressed_infos = Vec::with_capacity(1);

        // PDA 0: user_record
        // Get account info early before any mutable borrows
        let __account_info_0 = self.user_record.to_account_info();
        let __account_key_0 = *__account_info_0.key;

        // Extract address tree pubkey
        let __address_tree_pubkey_0: solana_pubkey::Pubkey = {
            use light_sdk::light_account_checks::AccountInfoTrait;
            let tree_info: &::light_sdk::sdk_types::PackedAddressTreeInfo = &params.create_accounts_proof.address_tree_info;
            cpi_accounts
                .get_tree_account_info(tree_info.address_merkle_tree_pubkey_index as usize)?
                .pubkey()
        };

        // Initialize CompressionInfo in account data
        {
            use light_sdk::interface::LightAccount;
            use anchor_lang::AnchorSerialize;
            let current_slot = anchor_lang::solana_program::sysvar::clock::Clock::get()?.slot;
            let account_info = self.user_record.to_account_info();
            {
                let __account_data_0 = &mut *self.user_record;
                __account_data_0.set_decompressed(&compression_config_data, current_slot);
            }
            let mut data = account_info.try_borrow_mut_data()
                .map_err(|_| light_sdk::error::LightSdkError::ConstraintViolation)?;
            self.user_record.serialize(&mut &mut data[8..])
                .map_err(|_| light_sdk::error::LightSdkError::ConstraintViolation)?;
        }

        // Register compressed address
        {
            let tree_info: &::light_sdk::sdk_types::PackedAddressTreeInfo = &params.create_accounts_proof.address_tree_info;
            ::light_sdk::interface::prepare_compressed_account_on_init(
                &__account_key_0,
                &__address_tree_pubkey_0,
                tree_info,
                params.create_accounts_proof.output_state_tree_index,
                0u8,
                &crate::ID,
                &mut all_new_address_params,
                &mut all_compressed_infos,
            )?;
        }

        // Reimburse fee_payer for rent paid to Anchor
        {
            let __created_accounts: [solana_account_info::AccountInfo<'info>; 1] = [
                self.user_record.to_account_info()
            ];
            ::light_sdk::interface::reimburse_rent(
                &__created_accounts,
                &self.fee_payer.to_account_info(),
                &self.pda_rent_sponsor.to_account_info(),
                &crate::ID,
            )?;
        }

        // Execute Light System Program CPI
        use light_sdk::cpi::{InvokeLightSystemProgram, LightCpiInstruction};
        light_sdk::cpi::v2::LightSystemProgramCpi::new_cpi(
            crate::LIGHT_CPI_SIGNER,
            params.create_accounts_proof.proof.clone()
        )
            .with_new_addresses(&all_new_address_params)
            .with_account_infos(&all_compressed_infos)
            .invoke(cpi_accounts)?;

        Ok(true)
    }
}

#[automatically_derived]
impl<'info> light_sdk::interface::LightFinalize<'info, CreateParams> for CreateAccounts<'info> {
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
| Light Token Config | `light_token_config` |
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
- `token::seeds` and `token::owner_seeds` are always required
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

### Struct-Level Validation
- `#[instruction]` with no `#[light_account(init)]` fields causes a compile error
- `#[derive(LightAccounts)]` is only for instructions that create light accounts
- Mark-only fields (without `init`) don't count - they're for `#[light_program]` discovery

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
|-- mod.rs                     Module exports
|-- shared_utils.rs            Common utilities (MetaExpr, type helpers)
|-- light_account_keywords.rs  Keyword validation for #[light_account]
|
|-- accounts/                  #[derive(LightAccounts)] for ACCOUNTS structs
|   |-- mod.rs                 Entry point, exports derive_light_accounts()
|   |-- derive.rs              Orchestration: parse -> validate -> generate
|   |-- builder.rs             LightAccountsBuilder for code generation
|   |-- parse.rs               Delegates to unified parsing (type aliases for backwards compat)
|   |-- validation.rs          Struct-level validation rules
|   |-- light_account.rs       #[light_account] attribute parsing
|   |-- pda.rs                 PDA compression block generation
|   |-- mint.rs                Mint action CPI generation (CreateMintsCpi batching)
|   |-- token.rs               Token account and ATA CPI generation
|   +-- variant.rs             Variant enum generation for light_program
|
|-- account/                   #[derive(LightAccount)] for DATA structs
|   |-- mod.rs                 Entry point for trait derives
|   |-- derive.rs              LightAccount derive implementation
|   |-- traits.rs              Trait implementations (HasCompressionInfo, CompressAs, Compressible)
|   |-- validation.rs          Shared validation utilities
|   +-- utils.rs               Shared utilities
|
+-- parsing/                   Unified parsing infrastructure
    |-- mod.rs                 Module exports
    |-- accounts_struct.rs     ParsedAccountsStruct (unified parsing entry point)
    |-- crate_context.rs       Crate-wide module parsing for struct discovery
    |-- infra.rs               Infrastructure field classification
    +-- instruction_arg.rs     Instruction argument parsing
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
- Zero-copy accounts use Pod serialization, and Borsh for decompression
- Data types must implement `bytemuck::Pod`, `bytemuck::Zeroable` and `borsh::{Serialize, Deserialize}`
- Zero-copy is for performance-critical accounts with fixed layouts

### Required Usage
- `#[derive(LightAccounts)]` requires `#[light_account(init)]` fields when `#[instruction]` is present
- The derive macro is only for instructions that create light accounts (rent-free PDAs, mints, tokens, ATAs)
- Mark-only fields (without `init`) are for `#[light_program]` discovery, not `#[derive(LightAccounts)]`

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
