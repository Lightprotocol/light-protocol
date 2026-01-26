# LightAccounts Derive Macro

## Overview

`#[derive(LightAccounts)]` generates variant structs and lifecycle hooks for compressible accounts.

- **Custom PDAs**: Full variant generation (seeds + data + traits)
- **Token/ATA/Mint**: Only seeds generation (uses SDK pre-implemented types)

## Account Types

```rust
enum AccountType {
    Pda,
    PdaZeroCopy,
    Token,
    Ata,
    Mint,
}
```

| Type | Description | Data Struct | Variant Type |
|------|-------------|-------------|--------------|
| `Pda` | Custom PDA (borsh) | User-defined | Generated `XxxVariant` |
| `PdaZeroCopy` | Custom PDA (zero-copy) | User-defined | Generated `XxxVariant` |
| `Token` | SPL Token account | `TokenData` (SDK) | `TokenVariant<S>` (SDK) |
| `Ata` | Associated Token Account | `TokenData` (SDK) | `AtaVariant<S>` (SDK) |
| `Mint` | SPL Mint account | `MintData` (SDK) | `MintVariant<S>` (SDK) |

---

## Summary

| Account Type | Macro Generates | Variant Type |
|--------------|-----------------|--------------|
| `#[light_account(init)]` | Seeds + Variant + Traits | `UserRecordVariant` (generated) |
| `#[light_account(init, zero_copy)]` | Seeds + Variant + Traits | `UserRecordVariant` (generated) |
| `#[light_account(init, token)]` | Seeds + Generic Variant | `TokenVariant<VaultSeeds>` (SDK generic) |
| `#[light_account(init, ata)]` | Seeds + Generic Variant | `AtaVariant<...>` (SDK generic) |
| `#[light_account(init, mint)]` | Seeds + Generic Variant | `MintVariant<...>` (SDK generic) |


---

## Attribute Reference

### Pda

```rust
#[account(seeds = [...], bump)]
#[light_account(init)]
pub user_record: Account<'info, UserRecord>,
```

| Attribute | Required | Description |
|-----------|----------|-------------|
| `init` | Yes | Marks field as a rent-free PDA account |

Seeds are extracted from Anchor's `#[account(seeds = [...], bump)]`.

### PdaZeroCopy

```rust
#[account(seeds = [...], bump)]
#[light_account(init, zero_copy)]
pub user_record: AccountLoader<'info, UserRecord>,
```

| Attribute | Required | Description |
|-----------|----------|-------------|
| `init` | Yes | Marks field as a rent-free PDA account |
| `zero_copy` | Yes | Uses Pod serialization with AccountLoader |

### Token

```rust
#[light_account(init, token, token::mint = mint, token::owner = owner, token::authority = [b"vault", owner.key().as_ref()])]
pub vault: UncheckedAccount<'info>,
```

| Attribute | Required | Description |
|-----------|----------|-------------|
| `init` | Yes | Enables token account creation |
| `token` | Yes | Marks as token account |
| `token::mint = <expr>` | Yes | The mint account |
| `token::owner = <expr>` | Yes | The PDA that owns this token account |
| `token::authority = [...]` | Yes | Seed expressions for the PDA owner |
| `token::bump = <expr>` | No | Explicit bump (auto-derived if omitted) |

**Shorthand:** `token::mint`, `token::owner`, `token::bump` assume variables named `mint`, `owner`, `bump`.

### Ata (Associated Token Account)

```rust
#[light_account(init, associated_token, associated_token::mint = mint, associated_token::authority = authority)]
pub user_ata: UncheckedAccount<'info>,
```

| Attribute | Required | Description |
|-----------|----------|-------------|
| `init` | Yes | Enables ATA creation |
| `associated_token` | Yes | Marks as associated token account |
| `associated_token::mint = <expr>` | Yes | The mint for the ATA |
| `associated_token::authority = <expr>` | Yes | The owner of the ATA |
| `associated_token::bump = <expr>` | No | Explicit bump (auto-derived if omitted) |

**Shorthand:** `associated_token::mint`, `associated_token::authority`, `associated_token::bump` assume same-named variables.

### Mint

```rust
#[light_account(
    init,
    mint,
    mint::signer = mint_signer,
    mint::authority = authority,
    mint::decimals = 9,
    mint::seeds = &[b"mint", authority.key().as_ref()]
)]
pub my_mint: UncheckedAccount<'info>,
```

#### Required Attributes

| Attribute | Description |
|-----------|-------------|
| `init` | Enables mint creation |
| `mint` | Marks as compressed mint |
| `mint::signer = <expr>` | The mint signer account (seeds the mint PDA) |
| `mint::authority = <expr>` | The mint authority |
| `mint::decimals = <expr>` | Number of decimals |
| `mint::seeds = &[...]` | Base seed expressions for mint_signer PDA (without bump) |

#### Optional Attributes

| Attribute | Default | Description |
|-----------|---------|-------------|
| `mint::bump = <expr>` | Auto-derived | Explicit bump for mint_seeds |
| `mint::freeze_authority = <expr>` | None | The freeze authority |
| `mint::authority_seeds = &[...]` | None | Base seeds for authority PDA |
| `mint::authority_bump = <expr>` | Auto-derived | Bump for authority_seeds (requires authority_seeds) |
| `mint::rent_payment = <expr>` | 16 | Rent payment epochs for decompression |
| `mint::write_top_up = <expr>` | 766 | Write top-up lamports for decompression |

#### Token Metadata Extension (all-or-nothing)

| Attribute | Description |
|-----------|-------------|
| `mint::name = <expr>` | Token name (requires symbol, uri) |
| `mint::symbol = <expr>` | Token symbol (requires name, uri) |
| `mint::uri = <expr>` | Token URI (requires name, symbol) |
| `mint::update_authority = <expr>` | Update authority (requires name, symbol, uri) |
| `mint::additional_metadata = <expr>` | Additional metadata key-value pairs (requires name, symbol, uri) |

---

## Example

```rust
#[derive(LightAccounts)]
pub struct Create<'info> {
    // PDA
    #[account(seeds = [b"user", authority.key().as_ref(), params.owner.as_ref()], bump)]
    #[light_account(init)]
    pub user_record: Account<'info, UserRecord>,

    // Token
    #[light_account(init, token, token::mint = mint, token::owner = owner, token::authority = [b"vault", owner.key().as_ref()])]
    pub vault: UncheckedAccount<'info>,

    // Mint with metadata
    #[light_account(
        init,
        mint,
        mint::signer = mint_signer,
        mint::authority = authority,
        mint::decimals = 9,
        mint::seeds = &[b"mint", authority.key().as_ref()],
        mint::name = "My Token",
        mint::symbol = "MTK",
        mint::uri = "https://example.com/token.json"
    )]
    pub my_mint: UncheckedAccount<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,
}
```

---

## CreateAccountsProof Requirement

Instructions with `#[light_account(init)]` fields require `create_accounts_proof: CreateAccountsProof` as an instruction argument.

```rust
pub struct CreateAccountsProof {
    /// The validity proof.
    pub proof: ValidityProof,
    /// Single packed address tree info (all accounts use same tree).
    pub address_tree_info: PackedAddressTreeInfo,
    /// Output state tree index for new compressed accounts.
    pub output_state_tree_index: u8,
    /// State merkle tree index (needed for mint creation decompress validation).
    pub state_tree_index: Option<u8>,
    /// Offset in remaining_accounts where Light system accounts start.
    pub system_accounts_offset: u8,
}
```

Can be in params struct:
```rust
pub fn create(ctx: Context<Create>, params: CreateParams) -> Result<()>
// where CreateParams has create_accounts_proof field
```

Or directly in instruction:
```rust
pub fn create(ctx: Context<Create>, create_accounts_proof: CreateAccountsProof, owner: Pubkey) -> Result<()>
```

Position in struct doesn't matter.

---

## Traits

### LightAccountVariant

```rust
trait LightAccountVariant: Sized + Clone + AnchorSerialize + AnchorDeserialize {
    type Seeds;
    type Data: LightAccount;
    type Packed: PackedLightAccountVariant<Unpacked = Self>;

    const SEED_COUNT: usize;

    fn seeds(&self) -> &Self::Seeds;
    fn data(&self) -> &Self::Data;
    fn data_mut(&mut self) -> &mut Self::Data;

    fn seed_refs(&self) -> [&[u8]; Self::SEED_COUNT];

    fn derive_pda(&self, program_id: &Pubkey) -> (Pubkey, u8) {
        let seeds = self.seed_refs();
        Pubkey::find_program_address(&seeds, program_id)
    }

    fn pack(&self, accounts: &mut PackedAccounts, program_id: &Pubkey) -> Result<Self::Packed, ProgramError>;
}
```

### PackedLightAccountVariant

```rust
trait PackedLightAccountVariant: Sized + Clone + AnchorSerialize + AnchorDeserialize {
    type Unpacked: LightAccountVariant;

    fn bump(&self) -> u8;

    fn unpack(&self, accounts: &[AccountInfo]) -> Result<Self::Unpacked, ProgramError>;
}
```

### LightPreInit

```rust
trait LightPreInit<'info, P> {
    fn light_pre_init(
        &mut self,
        remaining_accounts: &[AccountInfo<'info>],
        params: &P,
    ) -> std::result::Result<bool, LightSdkError>;
}
```

### LightFinalize

```rust
trait LightFinalize<'info, P> {
    fn light_finalize(
        &mut self,
        remaining_accounts: &[AccountInfo<'info>],
        params: &P,
        has_pre_init: bool,
    ) -> std::result::Result<(), LightSdkError>;
}
```

---

## Generated For Custom PDAs

From `#[light_account(init)]` field:

### Seeds Structs

```rust
// Extracted from #[account(seeds = [b"user", authority.key(), params.owner])]
pub struct UserRecordSeeds {
    pub authority: Pubkey,
    pub owner: Pubkey,
}

pub struct PackedUserRecordSeeds {
    pub authority_idx: u8,
    pub owner_idx: u8,
    pub bump: u8,
}
```

### Variant Structs

```rust
pub struct UserRecordVariant {
    pub seeds: UserRecordSeeds,
    pub data: UserRecord,
}

pub struct PackedUserRecordVariant {
    pub seeds: PackedUserRecordSeeds,
    pub data: PackedUserRecord,
}
```

### Trait Implementations

```rust
impl LightAccountVariant for UserRecordVariant {
    type Seeds = UserRecordSeeds;
    type Data = UserRecord;
    type Packed = PackedUserRecordVariant;

    const SEED_COUNT: usize = 3;

    fn seeds(&self) -> &Self::Seeds { &self.seeds }
    fn data(&self) -> &Self::Data { &self.data }
    fn data_mut(&mut self) -> &mut Self::Data { &mut self.data }

    // Generated from seeds = [b"user", authority.key(), params.owner]
    fn seed_refs(&self) -> [&[u8]; 3] {
        [
            b"user",
            self.seeds.authority.as_ref(),
            self.seeds.owner.as_ref(),
        ]
    }

    fn pack(&self, accounts: &mut PackedAccounts, program_id: &Pubkey) -> Result<Self::Packed, ProgramError> {
        let (_, bump) = self.derive_pda(program_id);
        Ok(PackedUserRecordVariant {
            seeds: PackedUserRecordSeeds {
                authority_idx: accounts.insert_or_get(self.seeds.authority),
                owner_idx: accounts.insert_or_get(self.seeds.owner),
                bump,
            },
            data: self.data.pack(accounts)?,
        })
    }
}

impl PackedLightAccountVariant for PackedUserRecordVariant {
    type Unpacked = UserRecordVariant;

    fn bump(&self) -> u8 { self.seeds.bump }

    fn unpack(&self, accounts: &[AccountInfo]) -> Result<Self::Unpacked, ProgramError> {
        Ok(UserRecordVariant {
            seeds: UserRecordSeeds {
                authority: *accounts.get(self.seeds.authority_idx as usize)
                    .ok_or(ProgramError::InvalidAccountData)?.key,
                owner: *accounts.get(self.seeds.owner_idx as usize)
                    .ok_or(ProgramError::InvalidAccountData)?.key,
            },
            data: UserRecord::unpack(&self.data, accounts)?,
        })
    }
}
```

### CPI Signing Helper

```rust
impl PackedUserRecordVariant {
    // For CPI signing with bump
    fn seed_refs_with_bump(&self, accounts: &[AccountInfo]) -> Result<[&[u8]; 4], ProgramError> {
        let authority = accounts.get(self.seeds.authority_idx as usize)
            .ok_or(ProgramError::InvalidAccountData)?;
        let owner = accounts.get(self.seeds.owner_idx as usize)
            .ok_or(ProgramError::InvalidAccountData)?;
        Ok([
            b"user",
            authority.key.as_ref(),
            owner.key.as_ref(),
            &[self.seeds.bump],
        ])
    }
}
```

---

## Generated For Token/ATA/Mint

From `#[light_account(token)]`, `#[light_account(ata)]`, `#[light_account(mint)]` fields:

Only seeds are generated. The variant types are pre-implemented in SDK.

### Seeds Only

```rust
// Generated for #[light_account(token)] vault field
pub struct VaultSeeds {
    pub authority: Pubkey,
}

pub struct PackedVaultSeeds {
    pub authority_idx: u8,
    pub bump: u8,
}

impl TokenSeeds for VaultSeeds {
    const SEED_COUNT: usize = 2;

    fn seed_refs(&self) -> [&[u8]; 2] {
        [b"vault", self.authority.as_ref()]
    }
}
```

### Uses SDK Pre-implemented Types

```rust
// SDK provides:
pub struct TokenVariant<S: TokenSeeds> {
    pub seeds: S,
    pub data: TokenData,
}

impl<S: TokenSeeds> LightAccountVariant for TokenVariant<S> { ... }

pub struct AtaVariant<S: AtaSeeds> {
    pub seeds: S,
    pub data: TokenData,
}

impl<S: AtaSeeds> LightAccountVariant for AtaVariant<S> { ... }

pub struct MintVariant<S: MintSeeds> {
    pub seeds: S,
    pub data: MintData,
}

impl<S: MintSeeds> LightAccountVariant for MintVariant<S> { ... }

// Macro generates type aliases:
type VaultVariant = TokenVariant<VaultSeeds>;
type PackedVaultVariant = PackedTokenVariant<PackedVaultSeeds>;
```

---

## Generated Lifecycle Hooks

Only fields with `init` generate pre_init calls:

```rust
#[derive(LightAccounts)]
pub struct Create<'info> {
    #[light_account(init)]              // -> generates pre_init_pda call
    pub user_record: Account<'info, UserRecord>,

    #[light_account(init, token)]       // -> generates pre_init_token call
    pub vault: Account<'info, TokenAccount>,

    #[light_account(token)]             // -> NO pre_init (manual init by user)
    pub existing_token: Account<'info, TokenAccount>,
}
```

```rust
impl<'info> LightPreInit<'info, CreateParams> for Create<'info> {
    fn light_pre_init(
        &mut self,
        remaining_accounts: &[AccountInfo<'info>],
        params: &CreateParams,
    ) -> std::result::Result<bool, LightSdkError> {
        let proof = &params.create_accounts_proof;
        // CPI accounts constructed from remaining_accounts sliced at system_accounts_offset
        let system_accounts_offset = proof.system_accounts_offset as usize;
        let cpi_accounts = CpiAccounts::new(
            &self.fee_payer,
            &remaining_accounts[system_accounts_offset..],
            LIGHT_CPI_SIGNER,
        );

        // 1. Collect CompressedAccountInfo from each PDA
        let mut compressed_accounts = Vec::new();

        compressed_accounts.push(
            light_sdk::light_pre_init_pda::<UserRecordVariant>(&self.user_record)?
        );
        // Add more PDAs here...

        let num_pdas = compressed_accounts.len() as u8;

        // 2. Initialize all PDAs in a single CPI
        light_sdk::light_init_pdas(
            &compressed_accounts,
            proof,
            remaining_accounts,
            &cpi_accounts,
        )?;

        // 3. Mint pre_init needs preceding PDA count for output indexing
        light_sdk::light_pre_init_mint::<MyMintVariant>(
            &self.my_mint,
            proof,
            remaining_accounts,
            &cpi_accounts,
            num_pdas,
        )?;

        // 4. Token pre_init
        light_sdk::light_pre_init_token::<VaultVariant>(
            &self.vault,
            remaining_accounts,
            &cpi_accounts,
        )?;

        // existing_token: no pre_init (no `init` attribute)
        Ok(false) // Return true if mints were created and need CPI context execution
    }
}

impl<'info> LightFinalize<'info, CreateParams> for Create<'info> {
    fn light_finalize(
        &mut self,
        remaining_accounts: &[AccountInfo<'info>],
        params: &CreateParams,
        has_pre_init: bool,
    ) -> std::result::Result<(), LightSdkError> {
        // Finalize is Noop for PDA-only flows
        Ok(())
    }
}
```

---

## PreInit SDK Functions

The SDK provides generic pre_init functions for each account type. These are called by the generated `LightPreInit` implementation.

### `light_pre_init_pda`

```rust
pub fn light_pre_init_pda<V: LightAccountVariant>(
    account: &AccountInfo,
) -> Result<CompressedAccountInfo>
```

**What it does:**
1. Derives the compressed address from the PDA pubkey (address_seed = pda_pubkey.to_bytes())
2. Creates a `CompressedAccountInfo` with:
   - **Discriminator**: `[255, 255, 255, 255, 255, 255, 255, 0]` - marks this as a rent-free PDA placeholder
   - **Data**: PDA pubkey bytes (32 bytes) - allows lookup/verification by on-chain PDA address
   - **Data hash**: SHA256 hash of the PDA pubkey bytes
3. Returns the `CompressedAccountInfo` to be collected for batch initialization

### `light_init_pdas`

```rust
pub fn light_init_pdas(
    compressed_accounts: &[CompressedAccountInfo],
    proof: &CreateAccountsProof,
    remaining_accounts: &[AccountInfo],
    cpi_accounts: &CpiAccounts,
) -> Result<()>
```

**What it does:**
1. Takes all collected `CompressedAccountInfo` from `light_pre_init_pda` calls
2. Creates compressed accounts with:
   - Addresses derived from PDA pubkeys
   - Discriminator `[255, 255, 255, 255, 255, 255, 255, 0]` (rent-free PDA placeholder)
   - Data containing the PDA pubkey bytes (32 bytes)
3. Invokes Light System Program CPI with the proof and new addresses
4. Anchor handles creating and initializing the on-chain PDA accounts

### `light_pre_init_mint`

```rust
pub fn light_pre_init_mint<V: LightAccountVariant>(
    account: &AccountInfo,
    proof: &CreateAccountsProof,
    remaining_accounts: &[AccountInfo],
    cpi_accounts: &CpiAccounts,
    num_preceding_pdas: u8,
) -> Result<()>
```

**What it does:**
1. CPIs to the light token program's create mint instruction
2. Can create one or more mints in a single CPI
3. Initializes the compressed mint account with the provided configuration
4. Uses `proof` for the ZK proof required to create the compressed mint account
5. Uses `num_preceding_pdas` to correctly index into the output accounts (compressed PDAs are created first in the CPI context)

### `light_decompress_mints`

```rust
pub fn light_decompress_mints(
    mint_data: &[MintData],
    accounts: &[AccountInfo],
    proof: &CreateAccountsProof,
    remaining_accounts: &[AccountInfo],
    cpi_accounts: &CpiAccounts,
) -> Result<()>
```

**What it does:**
1. CPIs to the light token program to decompress all mints in a single call
2. Takes a slice of all mint data to decompress
3. Uses `proof` for the ZK proof required to create the compressed mint accounts

### `light_pre_init_token`

```rust
pub fn light_pre_init_token<V: LightAccountVariant>(
    account: &AccountInfo,
    remaining_accounts: &[AccountInfo],
    cpi_accounts: &CpiAccounts,
) -> Result<()>
```

**What it does:**
1. CPIs to the light token program's create token account instruction
2. Creates a compressed token account associated with the PDA

### `light_pre_init_ata`

```rust
pub fn light_pre_init_ata<V: LightAccountVariant>(
    account: &AccountInfo,
    remaining_accounts: &[AccountInfo],
    cpi_accounts: &CpiAccounts,
) -> Result<()>
```

**What it does:**
1. CPIs to the light token program's create associated token account instruction
2. Creates a compressed ATA derived from owner + mint

---

## Seed Extraction

Seeds are extracted from Anchor's `#[account(seeds = [...])]` attribute and classified into three categories:

| Category | Examples | Detection | Stored In |
|----------|----------|-----------|-----------|
| **Constant** | `b"user"`, `SEED`, `crate::SEED` | Literals, uppercase identifiers | `seed_refs()` directly |
| **Account** | `authority.key()`, `owner.key()` | Not in `#[instruction(...)]` args | `Seeds` struct as `Pubkey` |
| **InstructionData** | `owner`, `params.owner`, `amount.to_le_bytes()` | Matches `#[instruction(...)]` arg | `Seeds` struct (type varies) |

### Detection Order

1. Literals (`b"user"`) and uppercase (`SEED`) -> **Constant**
2. Root matches `#[instruction(...)]` arg name -> **InstructionData**
3. Fallback -> **Account** (assumed ctx field)

### No Prefix Required for Instruction Data

```rust
// Format 1: struct parameter
#[instruction(params: CreateParams)]
#[account(seeds = [b"user", params.owner.as_ref()], bump)]

// Format 2: individual parameters - no prefix needed
#[instruction(owner: Pubkey, amount: u64)]
#[account(seeds = [b"user", owner.as_ref()], bump)]  // bare 'owner' works
```

### Account Seeds Must Be Struct Fields

```rust
#[account(seeds = [b"vault", authority.key().as_ref()], bump)]
// authority must be a declared field in the Accounts struct
// remaining_accounts are NOT permitted as seeds
```

### Example

```rust
#[account(seeds = [b"user", authority.key().as_ref(), owner.as_ref()], bump)]
//                  ^^^^^   ^^^^^^^^^^^^^^^^^^^^^^^   ^^^^^^^^^^^^
//                  Constant Account -> Seeds.authority  InstructionData -> Seeds.owner
```

---


 # Missing (Need Implementation)

- light_pre_init_pda - SDK function
- light_init_pdas - SDK function
- light_pre_init_mint - SDK function
- light_decompress_mints - SDK function
- light_pre_init_token - SDK function
- light_pre_init_ata - SDK function
