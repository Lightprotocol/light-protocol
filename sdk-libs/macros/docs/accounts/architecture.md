# RentFree Derive Macro and Trait Derives

## 1. Overview

The `#[derive(LightAccounts)]` macro and associated trait derives enable rent-free compressed accounts on Solana with minimal boilerplate. These macros generate code for:

- Pre-instruction compression setup (`LightPreInit` trait)
- Post-instruction cleanup (`LightFinalize` trait)
- Account data serialization and hashing
- Pubkey compression to u8 indices

### 1.1 Module Structure

```
sdk-libs/macros/src/rentfree/
|-- mod.rs                    # Module exports
|-- shared_utils.rs           # Common utilities (constant detection, identifier extraction)
|
|-- accounts/                 # #[derive(LightAccounts)] for Accounts structs
|   |-- mod.rs                # Module entry point
|   |-- derive.rs             # Orchestration layer
|   |-- builder.rs            # Code generation builder
|   |-- parse.rs              # Attribute parsing with darling
|   |-- pda.rs                # PDA block code generation
|   +-- light_mint.rs         # Mint action CPI generation
|
+-- traits/                   # Trait derive macros for data structs
    |-- mod.rs                # Module entry point
    |-- traits.rs             # HasCompressionInfo, Compressible, CompressAs, Size
    |-- pack_unpack.rs        # Pack/Unpack traits with Packed struct generation
    |-- light_compressible.rs # Combined LightCompressible derive
    |-- seed_extraction.rs    # Anchor seed extraction from #[account(...)]
    |-- decompress_context.rs # Decompression context utilities
    +-- utils.rs              # Shared utilities (field extraction, type checks)
```

---

## 2. `#[derive(LightAccounts)]` Derive Macro

### 2.1 Purpose

Generates `LightPreInit` and `LightFinalize` trait implementations for Anchor Accounts structs. These traits enable automatic compression of PDA accounts and mint creation during instruction execution.

**Source**: `sdk-libs/macros/src/rentfree/accounts/derive.rs`

### 2.2 Supported Attributes

#### `#[light_account(init)]` - Mark PDA Fields for Compression

Applied to `Account<'info, T>` or `Box<Account<'info, T>>` fields.

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
    #[light_account(init)]  // Uses default address_tree_info and output_tree from params
    pub user_record: Account<'info, UserRecord>,
}
```

**Optional arguments**:
- `address_tree_info` - Expression of type `PackedAddressTreeInfo` containing packed tree indices (default: `params.create_accounts_proof.address_tree_info`). Note: If you have an `AddressTreeInfo` with Pubkeys, you must pack it client-side using `pack_address_tree_info()` before passing to the instruction.
- `output_tree` - Expression for output tree index (default: `params.create_accounts_proof.output_state_tree_index`)

```rust
#[rentfree(
    address_tree_info = custom_tree_info,
    output_tree = custom_output_index
)]
pub user_record: Account<'info, UserRecord>,
```

#### `#[light_account(init, mint,...)]` - Mark Mint Fields

Creates a compressed mint with automatic decompression.

```rust
#[light_account(init, mint,
    mint_signer = mint_signer,      // AccountInfo that seeds the mint PDA (required)
    authority = authority,           // Mint authority (required)
    decimals = 9,                    // Token decimals (required)
    mint_seeds = &[b"mint", &[bump]], // PDA signer seeds for mint_signer (required)
    freeze_authority = freeze_auth,  // Optional freeze authority
    authority_seeds = &[b"auth", &[auth_bump]], // PDA signer seeds for authority (optional - if not provided, authority must be a tx signer)
    rent_payment = 2,                // Rent payment epochs (default: 2)
    write_top_up = 0                 // Write top-up lamports (default: 0)
)]
pub mint: Account<'info, Mint>,
```

#### `#[instruction(...)]` - Specify Instruction Parameters (Required)

Must be present on the struct when using `#[light_account(init)]` or `#[light_account(init)]`.

```rust
#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateParams)]
pub struct CreateAccounts<'info> { ... }
```

### 2.3 Infrastructure Field Detection

Infrastructure fields are auto-detected by naming convention. No attribute required.

| Field Type | Accepted Names |
|------------|----------------|
| Fee Payer | `fee_payer`, `payer`, `creator` |
| Compression Config | `compression_config` |
| CToken Config | `light_token_compressible_config`, `ctoken_config`, `light_token_config_account` |
| CToken Rent Sponsor | `ctoken_rent_sponsor`, `light_token_rent_sponsor` |
| CToken Program | `ctoken_program`, `light_token_program` |
| CToken CPI Authority | `light_token_cpi_authority`, `light_token_program_cpi_authority`, `compress_token_program_cpi_authority` |

**Source**: `sdk-libs/macros/src/rentfree/accounts/parse.rs` (lines 30-53)

### 2.4 Code Generation Flow

```
1. Parse
   |-- parse_rentfree_struct() extracts:
   |   - Struct name and generics
   |   - #[light_account(init)] fields -> RentFreeField
   |   - #[light_account(init)] fields -> LightMintField
   |   - #[instruction] args
   |   - Infrastructure fields by naming convention
   |
2. Validate
   |-- Total fields <= 255 (u8 index limit)
   |-- #[instruction] required when #[light_account(init)] or #[light_account(init)] present
   |
3. Generate pre_init Body
   |-- PDAs + Mints: generate_pre_init_pdas_and_mints()
   |   - Write PDAs to CPI context
   |   - Invoke mint_action with decompress + CPI context
   |-- Mints only: generate_pre_init_mints_only()
   |-- PDAs only: generate_pre_init_pdas_only()
   |-- Neither: Ok(false)
   |
4. Wrap in Trait Impls
   |-- LightPreInit<'info, ParamsType>
   +-- LightFinalize<'info, ParamsType>
```

**Source**: `sdk-libs/macros/src/rentfree/accounts/derive.rs`

### 2.5 Generated Code Example

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

## 3. Trait Derives (traits/)

### 3.0 Trait Composition Overview

The following diagram shows how the derive macros compose together to enable rent-free compressed accounts:

```
                            ACCOUNT STRUCT LEVEL
                            ====================

                        +--------------------+
                        | #[derive(LightAccounts)]|  <-- Applied to Anchor Accounts struct
                        +--------------------+
                                 |
                                 | generates
                                 v
                   +---------------------------+
                   | LightPreInit + LightFinalize |
                   +---------------------------+
                                 |
                                 | uses traits from
                                 v
                            DATA STRUCT LEVEL
                            =================

+-------------------------------------------------------------------------+
|                      #[derive(LightCompressible)]                       |
|                   (convenience macro - expands to all below)            |
+-------------------------------------------------------------------------+
         |                    |                    |                    |
         | expands to         | expands to         | expands to         | expands to
         v                    v                    v                    v
+----------------+   +------------------+   +--------------+   +-----------------+
| LightHasherSha |   | LightDiscriminator|   | Compressible |   | CompressiblePack|
+----------------+   +------------------+   +--------------+   +-----------------+
         |                    |                    |                    |
         | generates          | generates          | generates          | generates
         v                    v                    v                    v
+----------------+   +------------------+   +--------------+   +-----------------+
| - DataHasher   |   | - LightDiscriminator|  | (see below)|   | - Pack          |
| - ToByteArray  |   |   (8-byte unique ID) |  |            |   | - Unpack        |
+----------------+   +------------------+   +--------------+   | - Packed{Name}  |
                                                   |           |   struct         |
                                                   v           +-----------------+
                                    +-----------------------------+
                                    |        Compressible         |
                                    |    (combined derive macro)  |
                                    +-----------------------------+
                                       |      |       |       |
                                       v      v       v       v
                            +------------------+  +------------------+
                            | HasCompressionInfo|  |    CompressAs   |
                            +------------------+  +------------------+
                            | - compression_info()| | - compress_as() |
                            | - compression_info_mut()| Creates compressed |
                            | - set_compression_info_none()| representation|
                            +------------------+  +------------------+
                                       |                   |
                                       v                   v
                            +------------------+  +------------------+
                            |       Size       |  | CompressedInitSpace|
                            +------------------+  +------------------+
                            | - size()         |  | - INIT_SPACE     |
                            | Serialized size  |  | Compressed account|
                            +------------------+  +------------------+


                          RELATIONSHIP SUMMARY
                          ====================

    +-------------------------------------------------------------------+
    |                     USER'S PROGRAM CODE                           |
    +-------------------------------------------------------------------+
    |                                                                   |
    |  // Data struct - apply LightCompressible                         |
    |  #[derive(LightCompressible)]                                     |
    |  #[account]                                                       |
    |  pub struct UserRecord {                                          |
    |      pub owner: Pubkey,                                           |
    |      pub score: u64,                                              |
    |      pub compression_info: Option<CompressionInfo>,  <-- Required |
    |  }                                                                |
    |                                                                   |
    |  // Accounts struct - apply RentFree                              |
    |  #[derive(Accounts, LightAccounts)]                                    |
    |  #[instruction(params: CreateParams)]                             |
    |  pub struct Create<'info> {                                       |
    |      #[account(init, ...)]                                        |
    |      #[light_account(init)]                       <-- Marks for compression  |
    |      pub user_record: Account<'info, UserRecord>,                 |
    |  }                                                                |
    |                                                                   |
    +-------------------------------------------------------------------+
                                    |
                                    | At runtime, RentFree uses traits from
                                    | LightCompressible to:
                                    v
    +-------------------------------------------------------------------+
    | 1. Hash account data (DataHasher, ToByteArray)                    |
    | 2. Get discriminator (LightDiscriminator)                         |
    | 3. Create compressed representation (CompressAs)                  |
    | 4. Calculate sizes (Size, CompressedInitSpace)                    |
    | 5. Pack Pubkeys to indices (Pack, Unpack)                         |
    | 6. Access compression info (HasCompressionInfo)                   |
    +-------------------------------------------------------------------+
```

### 3.1 HasCompressionInfo

Provides accessors for the `compression_info` field.

**Source**: `sdk-libs/macros/src/rentfree/traits/traits.rs` (lines 69-88)

**Requirements**: Struct must have `compression_info: Option<CompressionInfo>` field.

**Generated methods**:
- `compression_info(&self) -> &CompressionInfo`
- `compression_info_mut(&mut self) -> &mut CompressionInfo`
- `compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo>`
- `set_compression_info_none(&mut self)`

### 3.2 Compressible

Combined derive that generates:
- `HasCompressionInfo` - Accessor for compression_info field
- `CompressAs` - Creates compressed representation
- `Size` - Calculates serialized size
- `CompressedInitSpace` - INIT_SPACE for compressed accounts

**Source**: `sdk-libs/macros/src/rentfree/traits/traits.rs` (lines 233-272)

**Optional attribute** `#[compress_as(field = expr, ...)]`:
- Override field values in compressed representation
- Useful for zeroing out fields that shouldn't be hashed

```rust
#[derive(Compressible)]
#[compress_as(start_time = 0, cached_value = 0)]
pub struct GameSession {
    pub session_id: u64,
    pub player: Pubkey,
    pub start_time: u64,      // Will be 0 in compressed form
    pub cached_value: u64,    // Will be 0 in compressed form
    pub compression_info: Option<CompressionInfo>,
}
```

**Auto-skipped fields**:
- `compression_info` (always handled specially)
- Fields with `#[skip]` attribute

#### `#[skip]` - Exclude Fields from Compression

Mark fields to exclude from `CompressAs` and `Size` calculations:

```rust
#[derive(Compressible)]
pub struct CachedData {
    pub id: u64,
    #[skip]  // Not included in compressed representation
    pub cached_timestamp: u64,
    pub compression_info: Option<CompressionInfo>,
}
```

### 3.3 Pack/Unpack (CompressiblePack)

Generates `Pack` and `Unpack` traits with a `Packed{StructName}` struct where direct Pubkey fields are compressed to u8 indices.

**Source**: `sdk-libs/macros/src/rentfree/traits/pack_unpack.rs`

**Limitation**: Only direct `Pubkey` fields are converted to `u8` indices. `Option<Pubkey>` fields are **NOT** converted - they remain as `Option<Pubkey>` in the packed struct. This is because `Option<Pubkey>` can be `None`, which doesn't map cleanly to an index.

**Input**:
```rust
#[derive(CompressiblePack)]
pub struct UserRecord {
    pub owner: Pubkey,
    pub authority: Pubkey,
    pub score: u64,
    pub compression_info: Option<CompressionInfo>,
}
```

**Generated**:
```rust
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PackedUserRecord {
    pub owner: u8,           // Pubkey -> u8 index
    pub authority: u8,       // Pubkey -> u8 index
    pub score: u64,          // Non-Pubkey unchanged
    pub compression_info: Option<CompressionInfo>,
}

impl Pack for UserRecord {
    type Packed = PackedUserRecord;
    fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Self::Packed {
        PackedUserRecord {
            owner: remaining_accounts.insert_or_get(self.owner),
            authority: remaining_accounts.insert_or_get(self.authority),
            score: self.score,
            compression_info: None,
        }
    }
}

impl Unpack for PackedUserRecord {
    type Unpacked = UserRecord;
    fn unpack(&self, remaining_accounts: &[AccountInfo]) -> Result<Self::Unpacked, ProgramError> {
        Ok(UserRecord {
            owner: *remaining_accounts[self.owner as usize].key,
            authority: *remaining_accounts[self.authority as usize].key,
            score: self.score,
            compression_info: None,
        })
    }
}
```

**No Pubkey fields**: If struct has no Pubkey fields, generates identity implementations:
```rust
pub type PackedUserRecord = UserRecord;  // Type alias
// Pack::pack returns self.clone()
// Unpack::unpack returns self.clone()
```

### 3.4 LightCompressible

Convenience derive that combines all traits needed for a compressible account.

**Source**: `sdk-libs/macros/src/rentfree/traits/light_compressible.rs`

**Equivalent to**:
```rust
#[derive(LightHasherSha, LightDiscriminator, Compressible, CompressiblePack)]
```

**Generated traits**:
- `DataHasher` + `ToByteArray` (SHA256 hashing via LightHasherSha)
- `LightDiscriminator` (unique 8-byte discriminator)
- `HasCompressionInfo` + `CompressAs` + `Size` + `CompressedInitSpace` (via Compressible)
- `Pack` + `Unpack` + `Packed{Name}` struct (via CompressiblePack)

**Usage**:
```rust
#[derive(Default, Debug, InitSpace, LightCompressible)]
#[account]
pub struct UserRecord {
    pub owner: Pubkey,
    #[max_len(32)]
    pub name: String,
    pub score: u64,
    pub compression_info: Option<CompressionInfo>,
}
```

**Notes**:
- `compression_info` field is auto-detected and handled specially (no `#[skip]` needed)
- SHA256 hashes the entire struct via borsh serialization, so no `#[hash]` attributes needed
- **Important**: `compression_info` IS included in the hash. Set it to `None` before hashing for consistent results.

---

## 4. Source Code Structure

```
sdk-libs/macros/src/rentfree/
|
|-- mod.rs
|   Purpose: Module exports for rentfree macro system
|
|-- shared_utils.rs
|   Purpose: Common utilities shared across modules
|   Types:
|   - MetaExpr - darling wrapper for parsing Expr from attributes
|   Functions:
|   - qualify_type_with_crate(ty: &Type) -> Type - ensures crate:: prefix
|   - make_packed_type(ty: &Type) -> Option<Type> - creates Packed{Type} path
|   - make_packed_variant_name(variant_name: &Ident) -> Ident
|   - ident_to_type(ident: &Ident) -> Type
|   - is_constant_identifier(ident: &str) -> bool
|   - extract_terminal_ident(expr: &Expr, key_method_only: bool) -> Option<Ident>
|   - is_base_path(expr: &Expr, base: &str) -> bool
|
|-- accounts/
|   |-- mod.rs           Entry point, exports derive_rentfree()
|   |-- derive.rs        Orchestration: parse -> validate -> generate
|   |-- builder.rs       RentFreeBuilder for code generation
|   |-- parse.rs         Attribute parsing with darling
|   |   - ParsedRentFreeStruct
|   |   - RentFreeField (#[light_account(init)] data)
|   |   - InfraFields (auto-detected infrastructure)
|   |   - InfraFieldClassifier (naming convention matching)
|   |-- pda.rs           PDA compression block generation
|   |   - PdaBlockBuilder
|   |   - generate_pda_compress_blocks()
|   +-- light_mint.rs    Mint action CPI generation
|       - LightMintField (#[light_account(init)] data)
|       - InfraRefs - resolved infrastructure field references
|       - LightMintBuilder - builder pattern for mint CPI generation
|       - CpiContextParts - encapsulates CPI context branching logic
|
+-- traits/
    |-- mod.rs              Entry point for trait derives
    |-- traits.rs           Core traits
    |   - derive_has_compression_info()
    |   - derive_compress_as()
    |   - derive_compressible() [combined]
    |-- pack_unpack.rs      Pack/Unpack trait generation
    |   - derive_compressible_pack()
    |-- light_compressible.rs  Combined derive
    |   - derive_rentfree_account() [LightCompressible]
    |-- seed_extraction.rs  Anchor seed parsing
    |   - ClassifiedSeed enum
    |   - ExtractedSeedSpec, ExtractedTokenSpec
    |   - extract_anchor_seeds()
    |   - extract_account_inner_type()
    |-- decompress_context.rs  Decompression utilities
    +-- utils.rs            Shared utilities
        - extract_fields_from_derive_input()
        - is_copy_type(), is_pubkey_type()
```

---

## 5. Limitations

### Field Limits
- **Maximum 255 fields**: Total `#[light_account(init)]` + `#[light_account(init)]` fields must be <= 255 (u8 index limit)
- **Single mint field**: Currently only the first `#[light_account(init)]` field is processed

### Type Restrictions
- `#[light_account(init)]` only applies to `Account<'info, T>` or `Box<Account<'info, T>>` fields
- Nested `Box<Box<Account<...>>>` is not supported
- `#[light_account(init)]` and `#[light_account(init)]` are mutually exclusive on the same field

### No-op Fallback
When no `#[instruction]` attribute is present, the macro generates no-op implementations for backwards compatibility with non-compressible Accounts structs.

---

## 6. Related Documentation

- **`sdk-libs/macros/docs/light_program/`** - Program-level `#[light_program]` attribute macro (architecture.md + codegen.md)
- **`sdk-libs/macros/README.md`** - Package overview
- **`sdk-libs/sdk/`** - Runtime SDK with `LightPreInit`, `LightFinalize` trait definitions
