# Compressible SDK Overview

## What It Does

Compressible accounts = rent-free on-chain accounts. Hot when active, auto-compressed when cold. Single proof execution batches multiple account operations.

## Architecture

```
+--------------------+     +---------------------+     +------------------+
|   Client Layer     |     |   Macro Layer       |     |  Runtime Layer   |
| (compressible-     |---->| (#[compressible],   |---->| (decompress_     |
|  client)           |     |  #[LightFinalize])  |     |  runtime.rs)     |
+--------------------+     +---------------------+     +------------------+
        |                          |                          |
        v                          v                          v
   PackedAccounts          CompressedAccountVariant     CPI to ctoken/
   instruction             enum generation              light-system
   builders                seed providers
```

## Account Types

| Type | Declaration | Owner | Signing | Use Case |
|------|-------------|-------|---------|----------|
| **PDA** | `UserRecord = ("seed", ctx.x, data.y)` | Program | PDA seeds | State accounts |
| **CToken Vault** | `Vault = (is_token, "seed", ctx.cmint, authority = ("auth_seed"))` | Authority PDA | CPI seeds | Protocol vaults |
| **ATA** | `LightAta` (auto-included) | Wallet | TX signer | User token balances |
| **CMint** | `LightMint` (auto-included) | cToken program | Authority | Token mints |

## Macro Usage

### Program Module (`#[compressible]`)

```rust
#[compressible(
    // PDAs: (literal_seed, ctx.accounts.*, data.*)
    UserRecord = ("user", ctx.authority, data.owner),
    
    // CToken vaults: (is_token, seeds..., authority = seed_expr)
    Vault = (is_token, "vault", ctx.cmint, authority = ("vault_auth")),
    
    // Instruction data fields used in seeds
    owner = Pubkey,
)]
#[program]
pub mod my_program {
    // Auto-generated instructions:
    // - decompress_accounts_idempotent
    // - compress_accounts_idempotent
    // - initialize_compression_config
    // - update_compression_config
}
```

### Account Struct (`#[LightFinalize]`)

```rust
#[derive(Accounts, LightFinalize)]
pub struct CreatePoolState<'info> {
    #[account(init, payer = fee_payer, space = 8 + PoolState::INIT_SPACE)]
    #[compressible(
        address_tree_info = params.address_tree_info,
        output_tree = params.output_state_tree_index
    )]
    pub pool_state: Account<'info, PoolState>,
    
    #[light_mint(
        mint_signer = mint_signer,
        authority = mint_authority,
        decimals = 9,
        address_tree_info = params.mint_address_tree_info,
    )]
    pub cmint: UncheckedAccount<'info>,
}
```

## Execution Flow

### Decompression (Cold -> Hot)

```
CLIENT                          ON-CHAIN
------                          --------
1. Fetch compressed accounts    
2. get_validity_proof()         
3. Pack indices into            
   DecompressInput variants     
4. Build instruction            
                                5. Unpack indices -> pubkeys
                                6. Process by type:
                                   - PDAs: write to CPI context
                                   - Mints: separate CPI call
                                   - Tokens: batched Transfer2 CPI
                                7. Execute proof verification
```

### Compression (Hot -> Cold)

```
ON-CHAIN (after epoch expiry)
-----------------------------
1. Account becomes "cold" (rent expires)
2. Forester calls compress_accounts_idempotent
3. Account data hashed, stored in merkle tree
4. On-chain account closed, rent returned
```

## CPI Context Batching

Multiple operations share one proof via CPI context:

```
                    +-----------------+
                    | CPI Context     |
                    | Account         |
                    +-----------------+
                          ^
    +----------+          |          +-----------+
    | PDAs     |--WRITE---+---READ---| Tokens    |
    | (first)  |                     | (execute) |
    +----------+                     +-----------+
```

**Order matters:**
1. PDAs write to context (no on-chain modification)
2. Last operation executes and verifies proof

## Constraints

| Combination | Allowed | Reason |
|-------------|---------|--------|
| PDA + PDA | YES | PDAs write to CPI context |
| PDA + Token | YES | PDA writes, token executes |
| PDA + Mint | YES | PDA writes, mint executes |
| Token + Token | YES | Batched in one CPI |
| **Mint + Token** | **NO** | Both modify on-chain state |
| **>1 Mint** | **NO** | Mint ops not batchable |

## Client API

### DecompressInput Variants

```rust
pub enum DecompressInput<T> {
    // Program PDA with custom data
    ProgramData(CompressedAccount, T),
    
    // Standard ATA (wallet must sign TX)
    Ata {
        compressed_token: CompressedTokenAccount,
        wallet_owner: Pubkey,
    },
    
    // Standard CMint
    Mint {
        compressed_account: CompressedAccount,
        mint_seed_pubkey: Pubkey,
        rent_payment: u8,
        write_top_up: u32,
    },
}
```

### Building Instructions

```rust
// Unified builder for mixed types
let ix = decompress_accounts_unified::<MyVariant, SeedParams>(
    &program_id,
    &DISCRIMINATOR,
    &pda_addresses,
    inputs,              // Vec<DecompressInput<T>>
    &program_accounts,
    validity_proof,
    seed_params,
)?;

// PDA-only builder (simpler)
let ix = decompress_accounts_idempotent::<T, S>(
    &program_id,
    &DISCRIMINATOR,
    &pda_addresses,
    &compressed_accounts, // &[(CompressedAccount, T)]
    &program_accounts,
    validity_proof,
    seed_data,
)?;
```

## Packing System

Pubkeys -> indices for tx size efficiency:

```
BEFORE (full pubkeys):
[32 bytes][32 bytes][32 bytes][32 bytes] = 128 bytes

AFTER (indices into remaining_accounts):
[1 byte][1 byte][1 byte][1 byte] = 4 bytes

remaining_accounts: [Pubkey1, Pubkey2, ...]
                       ^0        ^1
```

## Generated Types

The macro generates:

```rust
// Enum of all compressible account types
pub enum CompressedAccountVariant {
    UserRecord(UserRecord),
    PackedUserRecord(PackedUserRecord),
    // ... for each declared type
    PackedCTokenData(PackedCTokenData<CTokenAccountVariant>),
    CTokenData(CTokenData<CTokenAccountVariant>),
    LightAta(LightAta),   // always included
    LightMint(LightMint), // always included
}

// CToken variant enum (from is_token declarations)
pub enum CTokenAccountVariant {
    Vault = 0,
    // ...
}

// Seed params for instruction data
pub struct SeedParams {
    pub owner: Pubkey,
    // ... from field declarations
}
```

## Limitations

1. **Max 1 mint per instruction** - Mint decompression not batchable
2. **Mint + Token forbidden** - Both modify on-chain state, can't share CPI context
3. **CPI depth** - Nested CPIs limited by Solana (4 levels)
4. **Account size** - Compressed account data size limits apply
5. **Tree capacity** - Merkle tree depth determines max leaves
6. **Proof size** - More accounts = larger validity proof

## Error Cases

| Error | Cause | Fix |
|-------|-------|-----|
| "At most 1 mint" | >1 LightMint in inputs | Split into separate txs |
| "Mint + ATA forbidden" | Mixed mint and token decompression | Use separate instructions |
| "Tree info mismatch" | Compressed accounts from different trees | Ensure consistent tree_info |
| "CPI context required" | Mixed types without CPI context | Add cpi_context to remaining_accounts |
| "Wallet not signer" | ATA decompression without wallet signing | Include wallet as TX signer |

## Files Reference

```
sdk-libs/
├── compressible-client/     # Client instruction builders
│   └── src/lib.rs           # DecompressInput, instruction builders
├── macros/src/compressible/ # Proc macro implementation
│   ├── instructions.rs      # add_compressible_instructions
│   ├── variant_enum.rs      # CompressedAccountVariant generation
│   └── seed_providers.rs    # PDA/CToken seed derivation
├── ctoken-sdk/src/
│   └── compressible/
│       └── decompress_runtime.rs  # On-chain decompression handlers
└── sdk/src/compressible/
    ├── standard_types.rs    # LightAta, LightMint
    └── decompress_runtime.rs # PDA decompression handlers
```
