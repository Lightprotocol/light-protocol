# Compressible Macro - Final Implementation

## Overview

The `#[compressible(...)]` macro generates all types and code needed for rent-free account compression/decompression. This document shows exactly what exists now and how to use it.

## Status: Complete (Phase 1-8)

All phases implemented and tested, including Phase 8 CToken seed refactor. See `csdk-anchor-full-derived-test` for working example.

### Phase 8 Key Changes

- `CTokenAccountVariant` now has struct variants with Pubkey fields for seeds
- `PackedCTokenAccountVariant` has struct variants with u8 idx fields
- `CTokenSeedProvider` trait no longer requires accounts struct parameter
- `DecompressAccountsIdempotent` no longer needs named seed accounts
- All seed resolution happens via variant idx fields and `post_system_accounts`

---

## 1. Macro Declaration

```rust
#[compressible(
    // PDA accounts with seeds
    UserRecord = (seeds = ("user_record", ctx.authority, ctx.mint_authority, data.owner, data.category_id.to_le_bytes())),
    GameSession = (seeds = (GAME_SESSION_SEED, ctx.user, ctx.authority, data.session_id.to_le_bytes())),

    // Token accounts (CTokens)
    Vault = (is_token, seeds = ("vault", ctx.cmint), authority = ("vault_authority")),

    // Instruction data fields (for data.* seeds)
    owner = Pubkey,
    category_id = u64,
    session_id = u64,
)]
pub mod my_program { ... }
```

### Seed Types

- `ctx.*` - Context accounts (Pubkeys from instruction accounts)
- `data.*` - Data fields (from compressed account data, verified at construction time)
- String literals - Static seeds
- Constants - e.g., `GAME_SESSION_SEED`

---

## 2. Generated Types

### 2.1 CompressedAccountVariant Enum (Struct Variants)

```rust
pub enum CompressedAccountVariant {
    // Unpacked variants (with ctx.* Pubkeys)
    UserRecord {
        data: UserRecord,
        authority: Pubkey,
        mint_authority: Pubkey,
    },
    GameSession {
        data: GameSession,
        user: Pubkey,
        authority: Pubkey,
    },

    // Packed variants (with u8 indices into remaining_accounts)
    PackedUserRecord {
        data: PackedUserRecord,
        authority_idx: u8,
        mint_authority_idx: u8,
    },
    PackedGameSession {
        data: PackedGameSession,
        user_idx: u8,
        authority_idx: u8,
    },

    // CToken variant (unchanged)
    CTokenData(CTokenData),
}
```

### 2.2 Seeds Structs (All Seeds - ctx._ + data._)

```rust
pub struct UserRecordSeeds {
    pub authority: Pubkey,       // ctx.authority
    pub mint_authority: Pubkey,  // ctx.mint_authority
    pub owner: Pubkey,           // data.owner (verified against account)
    pub category_id: u64,        // data.category_id (verified against account)
}

pub struct GameSessionSeeds {
    pub user: Pubkey,            // ctx.user
    pub authority: Pubkey,       // ctx.authority
    pub session_id: u64,         // data.session_id (verified against account)
}
```

### 2.3 Constructor Methods

```rust
impl CompressedAccountVariant {
    /// Deserializes data and verifies data.* seeds match.
    pub fn user_record(
        account_data: &[u8],
        seeds: UserRecordSeeds,
    ) -> Result<Self, Error> {
        let data = UserRecord::deserialize(&mut &account_data[..])?;

        // Verify data.* seeds match actual compressed data
        if data.owner != seeds.owner { return Err(SeedMismatch); }
        if data.category_id != seeds.category_id { return Err(SeedMismatch); }

        Ok(Self::UserRecord {
            data,
            authority: seeds.authority,
            mint_authority: seeds.mint_authority,
        })
    }

    pub fn game_session(
        account_data: &[u8],
        seeds: GameSessionSeeds,
    ) -> Result<Self, Error> { ... }
}
```

### 2.4 SeedParams Removed

`SeedParams` is no longer needed in instruction data. All seeds are now resolved:

- `ctx.*` seeds: From variant idx fields → resolved on-chain via `post_system_accounts`
- `data.*` seeds: From unpacked compressed account data (`self.field`)

---

## 3. Client API Types

### 3.1 AccountInterface

```rust
pub struct AccountInterface {
    pub pubkey: Pubkey,
    pub is_cold: bool,
    pub decompression_context: Option<PdaDecompressionContext>,
}

pub struct PdaDecompressionContext {
    pub compressed_account: CompressedAccount,
}

impl AccountInterface {
    pub fn cold(pubkey: Pubkey, compressed_account: CompressedAccount) -> Self;
    pub fn hot(pubkey: Pubkey) -> Self;
    pub fn compressed_data(&self) -> Option<&[u8]>;
}
```

### 3.2 RentFreeDecompressAccount

```rust
pub struct RentFreeDecompressAccount<V> {
    pub account_interface: AccountInterface,
    pub variant: V,
}
```

### 3.3 Instruction Builders

```rust
// Existing API (still works)
pub fn decompress_accounts_idempotent<T>(
    program_id: &Pubkey,
    discriminator: &[u8],
    decompressed_account_addresses: &[Pubkey],
    compressed_accounts: &[(CompressedAccount, T)],
    program_account_metas: &[AccountMeta],
    validity_proof_with_context: ValidityProofWithContext,
) -> Result<Instruction, Error>

// New API (filters cold accounts automatically)
pub fn decompress_accounts_idempotent_new<V>(
    program_id: &Pubkey,
    accounts: Vec<RentFreeDecompressAccount<V>>,
    program_account_metas: &[AccountMeta],
    validity_proof_with_context: ValidityProofWithContext,
    discriminator: Option<&[u8]>,
) -> Result<Option<Instruction>, Error>  // Returns None if no cold accounts
```

---

## 4. Client Usage (Complete Example)

From `csdk-anchor-full-derived-test/tests/basic_test.rs`:

```rust
use csdk_anchor_full_derived_test::{CTokenAccountVariant, CompressedAccountVariant};
use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::{
    GameSessionSeeds, UserRecordSeeds,  // NO SeedParams needed
};
use light_compressible_client::{
    compressible_instruction, AccountInterface, RentFreeDecompressAccount,
};

// 1. Fetch compressed accounts from indexer
let compressed_user = rpc
    .get_compressed_account(user_compressed_address, None)
    .await?
    .value
    .unwrap();

let compressed_game = rpc
    .get_compressed_account(game_compressed_address, None)
    .await?
    .value
    .unwrap();

// 2. Fetch compressed token accounts
let compressed_vault_accounts = rpc
    .get_compressed_token_accounts_by_owner(&vault_pda, None, None)
    .await?
    .value
    .items;
let compressed_vault = &compressed_vault_accounts[0];

// 3. Get validity proof for all accounts
let rpc_result = rpc
    .get_validity_proof(
        vec![
            compressed_user.hash,
            compressed_game.hash,
            compressed_vault.account.hash,
        ],
        vec![],
        None,
    )
    .await?
    .value;

// 4. Create AccountInterface for each cold account (from RPC response)
let user_interface = AccountInterface::cold(user_record_pda, compressed_user.clone());
let game_interface = AccountInterface::cold(game_session_pda, compressed_game.clone());
let vault_interface = AccountInterface::cold(vault_pda, compressed_vault.account.clone());

// 5. Construct variants using generated constructors (verifies data.* seeds match)
let user_variant = CompressedAccountVariant::user_record(
    user_interface.compressed_data().unwrap(),
    UserRecordSeeds {
        authority: authority.pubkey(),
        mint_authority: mint_authority.pubkey(),
        owner,           // Must match compressed data
        category_id,     // Must match compressed data
    },
).expect("UserRecord seed verification failed");

let game_variant = CompressedAccountVariant::game_session(
    game_interface.compressed_data().unwrap(),
    GameSessionSeeds {
        user: payer.pubkey(),
        authority: authority.pubkey(),
        session_id,      // Must match compressed data
    },
).expect("GameSession seed verification failed");

let vault_ctoken_data = light_ctoken_sdk::compat::CTokenData {
    variant: CTokenAccountVariant::Vault,
    token_data: compressed_vault.token.clone(),
};

// 6. Build RentFreeDecompressAccount for each account
let decompress_accounts = vec![
    RentFreeDecompressAccount::new(user_interface, user_variant),
    RentFreeDecompressAccount::new(game_interface, game_variant),
    RentFreeDecompressAccount::new(
        vault_interface,
        CompressedAccountVariant::CTokenData(vault_ctoken_data),
    ),
];

// 7. Build decompress instruction using NEW API - NO SeedParams or seed accounts needed!
let decompress_instruction = compressible_instruction::decompress_accounts_idempotent_new(
    &program_id,
    decompress_accounts,
    compressible_instruction::decompress::accounts(payer.pubkey(), config_pda, payer.pubkey()),
    rpc_result,
)?
.expect("Should have cold accounts to decompress");

// 8. Send transaction - done!
rpc.create_and_send_transaction(&[decompress_instruction], &payer.pubkey(), &[&payer])
    .await?;
```

---

## 5. On-Chain Flow

### 5.1 What Happens When Instruction Executes

```
1. UNPACK
   PackedUserRecord { data, authority_idx: 3, mint_authority_idx: 5 }
   -> authority = remaining_accounts[3].key
   -> mint_authority = remaining_accounts[5].key
   -> UserRecord { data, authority, mint_authority }

2. DERIVE PDA
   UserRecordCtxSeeds { authority, mint_authority }
   +
   self (unpacked UserRecord)
   -> seeds = ["user_record", authority, mint_authority, self.owner, self.category_id.to_le_bytes()]
   -> derived_pda = Pubkey::find_program_address(&seeds, program_id)

3. VERIFY
   assert!(derived_pda == target_solana_account.key)

4. CREATE/WRITE
   if !account_exists {
       create_pda(derived_pda)
   }
   write_data(data)
```

### 5.2 CPI Context Batching (Mixed PDAs + Tokens)

```
CRITICAL: When has_pdas && has_tokens:
1. PDAs FIRST: LightSystemProgramCpi.write_to_cpi_context_first()
2. Tokens LAST: invoke() with cpi_context (consumes context)

Client must use FIRST TOKEN's cpi_context when packing.
```

---

## 6. Key Implementation Details

### 6.1 Seed Resolution

| Seed Type | Where Stored           | Resolved At                              |
| --------- | ---------------------- | ---------------------------------------- |
| `ctx.*`   | Variant idx field (u8) | On-chain via `post_system_accounts[idx]` |
| `data.*`  | Unpacked account data  | On-chain via `self.field`                |
| Literals  | Hardcoded in macro     | Compile time                             |
| Constants | Hardcoded in macro     | Compile time                             |

### 6.2 Index Space

All indices (`authority_idx`, `mint_authority_idx`, etc.) reference `remaining_accounts` after system accounts:

```
remaining_accounts layout:
[0..system_end]: System accounts (light_system_program, etc.)
[system_end..tail_start]: Packed pubkeys (deduped)
[tail_start..]: Decompressed PDA addresses
```

### 6.3 Pack/Unpack Flow

```rust
// Client: Pack (Pubkey -> u8)
CompressedAccountVariant::UserRecord { data, authority, mint_authority }
-> Pack::pack(&mut remaining_accounts)
-> CompressedAccountVariant::PackedUserRecord {
       data: data.pack(&mut remaining_accounts),
       authority_idx: remaining_accounts.insert_or_get(authority),
       mint_authority_idx: remaining_accounts.insert_or_get(mint_authority),
   }

// On-chain: Unpack (u8 -> Pubkey)
PackedUserRecord { data, authority_idx, mint_authority_idx }
-> Unpack::unpack(post_system_accounts)
-> UserRecord {
       data: data.unpack(post_system_accounts)?,
       authority: *post_system_accounts[authority_idx].key,
       mint_authority: *post_system_accounts[mint_authority_idx].key,
   }
```

---

## 7. Files Reference

| File                                                     | Purpose                                           |
| -------------------------------------------------------- | ------------------------------------------------- |
| `sdk-libs/macros/src/compressible/instructions.rs`       | Main macro, generates seeds structs, constructors |
| `sdk-libs/macros/src/compressible/variant_enum.rs`       | CompressedAccountVariant enum, Pack/Unpack        |
| `sdk-libs/macros/src/compressible/decompress_context.rs` | DecompressContext trait impl                      |
| `sdk-libs/macros/src/compressible/seed_providers.rs`     | CToken seed provider (unchanged)                  |
| `sdk-libs/compressible-client/src/lib.rs`                | Client API types and instruction builders         |
| `sdk-tests/csdk-anchor-full-derived-test/`               | Complete working example                          |

---

## 8. Error Codes

```rust
pub enum CompressibleInstructionError {
    InvalidRentSponsor,
    MissingSeedAccount,
    SeedMismatch,  // data.* seeds don't match compressed account data
    CTokenDecompressionNotImplemented,
    PdaDecompressionNotImplemented,
    TokenCompressionNotImplemented,
    PdaCompressionNotImplemented,
}
```

---

## 9. Test Command

```bash
cargo test-sbf -p csdk-anchor-full-derived-test
```

---

## 10. Architecture Diagram

```
┌────────────────────────────────────────────────────────────────────────────┐
│                              CLIENT SIDE                                    │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────┐    ┌──────────────────────────────────────┐   │
│  │  RPC: get_compressed_   │    │         UserRecordSeeds              │   │
│  │       account()         │    │  ┌────────────────────────────────┐  │   │
│  │  ───────────────────►   │    │  │ authority: Pubkey     (ctx.*)  │  │   │
│  │  CompressedAccount {    │    │  │ mint_authority: Pubkey(ctx.*)  │  │   │
│  │    data: bytes,         │    │  │ owner: Pubkey       (data.*)   │  │   │
│  │    hash,                │    │  │ category_id: u64    (data.*)   │  │   │
│  │    tree_info,           │    │  └────────────────────────────────┘  │   │
│  │  }                      │    └──────────────────────────────────────┘   │
│  └─────────────────────────┘                    │                          │
│              │                                   │                          │
│              ▼                                   │                          │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │  AccountInterface::cold(pda_address, compressed_account)              │ │
│  │    - pubkey: Pubkey (target PDA address)                              │ │
│  │    - is_cold: true                                                    │ │
│  │    - decompression_context: Some(compressed_account)                  │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│              │                                   │                          │
│              └───────────────┬───────────────────┘                          │
│                              ▼                                              │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │  CompressedAccountVariant::user_record(interface.compressed_data(), seeds)│
│  │    1. Deserialize: UserRecord::deserialize(&data_bytes)               │ │
│  │    2. Verify: data.owner == seeds.owner                               │ │
│  │    3. Verify: data.category_id == seeds.category_id                   │ │
│  │    4. Return: UserRecord { data, authority, mint_authority }          │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                              │                                              │
│                              ▼                                              │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │  RentFreeDecompressAccount::new(account_interface, variant)           │ │
│  │    - account_interface: AccountInterface (pubkey + compressed data)   │ │
│  │    - variant: CompressedAccountVariant::UserRecord { ... }            │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                              │                                              │
│                              ▼                                              │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │  decompress_accounts_idempotent_new(                                  │ │
│  │      program_id,                                                      │ │
│  │      vec![decompress_account1, decompress_account2, ...],             │ │
│  │      &account_metas,                                                  │ │
│  │      validity_proof,                                                  │ │
│  │      None,  // default discriminator                                  │ │
│  │  )                                                                    │ │
│  │                                                                       │ │
│  │  1. Filter: keep only is_cold accounts                                │ │
│  │  2. Extract: pubkeys from account_interface                           │ │
│  │  3. Pack::pack() converts Pubkeys to indices                          │ │
│  │  4. Return: Some(Instruction) or None if all hot                      │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                              │                                              │
└──────────────────────────────┼──────────────────────────────────────────────┘
                               │
         ══════════════════════╪══════════════════════  TRANSACTION
                               │
                               ▼
┌────────────────────────────────────────────────────────────────────────────┐
│                              ON-CHAIN                                       │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │  1. UNPACK (PackedUserRecord → UserRecord)                            │ │
│  │     authority = post_system_accounts[authority_idx].key               │ │
│  │     mint_authority = post_system_accounts[mint_authority_idx].key     │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                              │                                              │
│                              ▼                                              │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │  2. DERIVE PDA                                                        │ │
│  │     ctx_seeds = UserRecordCtxSeeds { authority, mint_authority }      │ │
│  │     seeds = ["user_record",                                           │ │
│  │              ctx_seeds.authority,                                     │ │
│  │              ctx_seeds.mint_authority,                                │ │
│  │              self.owner,           // from unpacked data              │ │
│  │              self.category_id]     // from unpacked data              │ │
│  │     derived_pda = find_program_address(seeds, program_id)             │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                              │                                              │
│                              ▼                                              │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │  3. VERIFY & CREATE                                                   │ │
│  │     assert!(derived_pda == target_account.key)                        │ │
│  │     if !exists { create_pda() }                                       │ │
│  │     write_data(unpacked_data)                                         │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## PHASE 8: CToken Seed Refactor (COMPLETED)

### Current CToken Flow (Problem)

```
Client:
  CTokenAccountVariant::Vault  // No fields - just an enum tag
  + TokenData { owner, mint, amount }
  → Pack: variant just CLONED (no packing)

On-chain:
  CTokenSeedProvider::get_seeds(ctx.accounts, remaining_accounts)
    → ctx.accounts.cmint.as_ref()?.key()  // READS FROM NAMED ACCOUNT!
    → derive PDA with ["vault", cmint]
```

**Problem**: CToken seed resolution still requires named accounts in `DecompressAccountsIdempotent`.

### Target CToken Flow

```
Client:
  CTokenAccountVariant::Vault { cmint: Pubkey }  // HAS SEED FIELD!
  + TokenData { owner, mint, amount }
  → Pack: variant.cmint → cmint_idx (pushed to remaining_accounts)

On-chain:
  Unpack: cmint_idx → post_system_accounts[cmint_idx].key → cmint Pubkey
  CTokenSeedProvider::get_seeds(program_id)
    → self.cmint  // READS FROM VARIANT DIRECTLY!
    → derive PDA with ["vault", cmint]
```

**Result**: No named seed accounts needed. Same pattern as PDAs.

### Generated Types (After Refactor)

```rust
// Unpacked (client-side, with Pubkeys)
pub enum CTokenAccountVariant {
    Vault { cmint: Pubkey },
    UserAta { owner: Pubkey, cmint: Pubkey },  // If defined
}

// Packed (wire format, with indices)
pub enum PackedCTokenAccountVariant {
    Vault { cmint_idx: u8 },
    UserAta { owner_idx: u8, cmint_idx: u8 },
}

// Pack impl
impl Pack for CTokenAccountVariant {
    type Packed = PackedCTokenAccountVariant;

    fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Self::Packed {
        match self {
            CTokenAccountVariant::Vault { cmint } => {
                PackedCTokenAccountVariant::Vault {
                    cmint_idx: remaining_accounts.insert_or_get(*cmint),
                }
            }
        }
    }
}

// Unpack impl
impl Unpack for PackedCTokenAccountVariant {
    type Unpacked = CTokenAccountVariant;

    fn unpack(&self, remaining_accounts: &[AccountInfo]) -> Result<Self::Unpacked> {
        match self {
            PackedCTokenAccountVariant::Vault { cmint_idx } => {
                Ok(CTokenAccountVariant::Vault {
                    cmint: *remaining_accounts[*cmint_idx as usize].key,
                })
            }
        }
    }
}
```

### CTokenSeedProvider Trait Change

```rust
// BEFORE (requires accounts struct)
pub trait CTokenSeedProvider: Copy {
    type Accounts<'info>;

    fn get_seeds<'a, 'info>(
        &self,
        accounts: &'a Self::Accounts<'info>,  // Used for ctx.accounts.cmint
        remaining_accounts: &'a [AccountInfo<'info>],
    ) -> Result<(Vec<Vec<u8>>, Pubkey), ProgramError>;
}

// AFTER (self-contained)
pub trait CTokenSeedProvider: Copy {
    fn get_seeds(&self, program_id: &Pubkey) -> Result<(Vec<Vec<u8>>, Pubkey), ProgramError>;
    fn get_authority_seeds(&self, program_id: &Pubkey) -> Result<(Vec<Vec<u8>>, Pubkey), ProgramError>;
}
```

### Generated CTokenSeedProvider impl

```rust
impl CTokenSeedProvider for CTokenAccountVariant {
    fn get_seeds(&self, program_id: &Pubkey) -> Result<(Vec<Vec<u8>>, Pubkey), ProgramError> {
        match self {
            CTokenAccountVariant::Vault { cmint } => {
                // cmint is already resolved Pubkey from variant!
                let seeds: &[&[u8]] = &[b"vault", cmint.as_ref()];
                let (pda, bump) = Pubkey::find_program_address(seeds, program_id);
                let mut seeds_vec = seeds.iter().map(|s| s.to_vec()).collect::<Vec<_>>();
                seeds_vec.push(vec![bump]);
                Ok((seeds_vec, pda))
            }
        }
    }
}
```

### DecompressAccountsIdempotent Simplification

```rust
// BEFORE
pub struct DecompressAccountsIdempotent<'info> {
    pub fee_payer: Signer<'info>,
    pub config: AccountInfo<'info>,
    pub rent_sponsor: UncheckedAccount<'info>,
    // CToken static accounts
    pub ctoken_rent_sponsor: Option<AccountInfo<'info>>,
    pub ctoken_program: Option<UncheckedAccount<'info>>,
    pub ctoken_cpi_authority: Option<UncheckedAccount<'info>>,
    pub ctoken_config: Option<UncheckedAccount<'info>>,
    // SEED ACCOUNTS (needed by CTokenSeedProvider)
    pub authority: Option<UncheckedAccount<'info>>,
    pub mint_authority: Option<UncheckedAccount<'info>>,
    pub user: Option<UncheckedAccount<'info>>,
    pub cmint: Option<UncheckedAccount<'info>>,
    pub some_account: Option<UncheckedAccount<'info>>,
}

// AFTER
pub struct DecompressAccountsIdempotent<'info> {
    pub fee_payer: Signer<'info>,
    pub config: AccountInfo<'info>,
    pub rent_sponsor: UncheckedAccount<'info>,
    // CToken static accounts
    pub ctoken_rent_sponsor: Option<AccountInfo<'info>>,
    pub ctoken_program: Option<UncheckedAccount<'info>>,
    pub ctoken_cpi_authority: Option<UncheckedAccount<'info>>,
    pub ctoken_config: Option<UncheckedAccount<'info>>,
    // NO SEED ACCOUNTS - they're in the variant!
}
```

### Client Usage (After Refactor)

```rust
// Construct CToken variant with seed pubkeys
let vault_variant = CTokenAccountVariant::Vault { cmint: cmint_pda };
let ctoken_data = CTokenData {
    variant: vault_variant,
    token_data: compressed_vault.token.clone(),
};

let decompress_instruction = compressible_instruction::decompress_accounts_idempotent_new(
    &program_id,
    vec![
        RentFreeDecompressAccount::new(user_interface, user_variant),
        RentFreeDecompressAccount::new(vault_interface, CompressedAccountVariant::CTokenData { data: ctoken_data }),
    ],
    compressible_instruction::decompress::accounts(payer.pubkey(), config_pda, payer.pubkey()),
    rpc_result,
)?;
```

### decompress::accounts Helper

```rust
/// Returns program account metas for decompress_accounts_idempotent with CToken support.
/// Includes ctoken_rent_sponsor, ctoken_program, ctoken_cpi_authority, ctoken_config.
pub fn accounts(fee_payer: Pubkey, config: Pubkey, rent_sponsor: Pubkey) -> Vec<AccountMeta>;

/// Returns program account metas for PDA-only decompression (no CToken accounts).
pub fn accounts_pda_only(fee_payer: Pubkey, config: Pubkey, rent_sponsor: Pubkey) -> Vec<AccountMeta>;
```

### SDK Changes Required

| File                                                | Changes                                            |
| --------------------------------------------------- | -------------------------------------------------- |
| `ctoken-sdk/src/pack.rs`                            | Add Pack bound to V, use V::Packed for packed type |
| `sdk/src/compressible/decompress_runtime.rs`        | Update CTokenSeedProvider trait signature          |
| `macros/src/compressible/variant_enum.rs`           | Generate CTokenAccountVariant with struct fields   |
| `macros/src/compressible/seed_providers.rs`         | Update get_seeds to use self.field                 |
| `macros/src/compressible/instructions.rs`           | Remove seed account fields from Accounts struct    |
| `ctoken-sdk/src/compressible/decompress_runtime.rs` | Update process_decompress_tokens_runtime           |

### Flow Diagram (After Phase 8)

```
┌────────────────────────────────────────────────────────────────────────────┐
│                              CLIENT SIDE                                    │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  CTokenAccountVariant::Vault { cmint: cmint_pda }                          │
│  + TokenData { owner, mint, amount }                                        │
│  = CTokenData { variant, token_data }                                       │
│                              │                                              │
│                              ▼                                              │
│  Pack::pack()                                                               │
│    variant.cmint → cmint_idx = remaining_accounts.insert_or_get(cmint)     │
│    token_data.owner → owner_idx                                            │
│    token_data.mint → mint_idx                                              │
│  = PackedCTokenData { variant: Vault { cmint_idx }, token_data }           │
│                              │                                              │
└──────────────────────────────┼──────────────────────────────────────────────┘
                               │
         ══════════════════════╪══════════════════════  TRANSACTION
                               │
                               ▼
┌────────────────────────────────────────────────────────────────────────────┐
│                              ON-CHAIN                                       │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  PackedCTokenData { variant: Vault { cmint_idx }, token_data }             │
│                              │                                              │
│                              ▼                                              │
│  Unpack::unpack(post_system_accounts)                                       │
│    cmint = post_system_accounts[cmint_idx].key                              │
│    owner = post_system_accounts[owner_idx].key                              │
│  = CTokenData { variant: Vault { cmint }, token_data }                     │
│                              │                                              │
│                              ▼                                              │
│  CTokenSeedProvider::get_seeds(program_id)                                  │
│    match self {                                                             │
│      Vault { cmint } => seeds = ["vault", cmint.as_ref()]                  │
│    }                                                                        │
│  = (seeds, derived_pda)                                                     │
│                              │                                              │
│                              ▼                                              │
│  Verify: derived_pda == target_account.key                                  │
│  Create token account with seeds                                            │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

### Implementation Steps

1. **Update SDK trait** (`sdk/src/compressible/decompress_runtime.rs`):
   - Change `CTokenSeedProvider` signature to not require `accounts` param

2. **Update ctoken-sdk Pack** (`ctoken-sdk/src/pack.rs`):
   - Add `Pack` trait bound to `V` in `CTokenDataWithVariant<V>`
   - Use `V::Packed` as the packed variant type

3. **Generate CToken variant enums** (`variant_enum.rs`):
   - Parse token_seeds to extract ctx.\* fields
   - Generate `CTokenAccountVariant` with struct variants (Pubkeys)
   - Generate `PackedCTokenAccountVariant` with struct variants (indices)
   - Generate Pack/Unpack impls

4. **Update seed provider generation** (`seed_providers.rs`):
   - Change `get_seeds()` to use `self.cmint` instead of `ctx.accounts.cmint`

5. **Remove seed accounts** (`instructions.rs`):
   - Remove seed account fields from `DecompressAccountsIdempotent`

6. **Update tests** (`basic_test.rs`):
   - Construct `CTokenAccountVariant::Vault { cmint }` with Pubkey
   - Remove seed accounts from instruction building
