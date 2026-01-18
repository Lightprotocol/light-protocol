# Compressible Program SDK - Implementation Spec

## Account Types

```
┌───────────────────┬──────────────────────────────────┬────────────────────────┬─────────────┐
│ TYPE              │ MARKED WITH                      │ SEEDS NEEDED           │ DECOMPRESS  │
├───────────────────┼──────────────────────────────────┼────────────────────────┼─────────────┤
│ PDA               │ #[rentfree]                      │ Account seeds          │ batched     │
│                   │                                  │ (from #[account])      │ idempotent  │
├───────────────────┼──────────────────────────────────┼────────────────────────┼─────────────┤
│ Program Token     │ #[rentfree_token(authority=[..])]│ Token account seeds +  │ batched     │
│                   │                                  │ Authority PDA seeds    │ idempotent  │
├───────────────────┼──────────────────────────────────┼────────────────────────┼─────────────┤
│ ATA               │ (standard SPL)                   │ owner + mint           │ N×create +  │
│                   │                                  │                        │ 1×Transfer2 │
├───────────────────┼──────────────────────────────────┼────────────────────────┼─────────────┤
│ Mint              │ #[light_mint]                    │ mint_signer            │ 1 per mint  │
└───────────────────┴──────────────────────────────────┴────────────────────────┴─────────────┘
```

---

## Core Types

```rust
pub struct AllSpecs<V> {
    pub program_owned: Vec<ProgramOwnedSpec<V>>,  // PDAs + program-owned tokens
    pub atas: Vec<AtaSpec>,
    pub mints: Vec<MintSpec>,
}

pub enum Operation {
    Swap,
    Deposit,
    Withdraw,
    // Program-specific variants
}
```

---

## build_load_instructions Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         build_load_instructions<V>                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  1. FILTER COLD                                                             │
│     cold_program_owned = specs.program_owned.filter(|s| s.is_cold)          │
│       ↳ includes PDAs + program-owned tokens (both RentFreeDecompressAccount<V>)
│     cold_atas  = specs.atas.filter(|s| s.is_cold)                           │
│     cold_mints = specs.mints.filter(|m| m.is_cold)                          │
│     if all_empty → return []                                                │
│                                                                             │
│  2. FETCH PROOFS (concurrent)                                               │
│     program_proof = get_validity_proof(program_owned_hashes)                │
│     ata_proof     = get_validity_proof(ata_hashes)                          │
│     mint_proofs   = [get_validity_proof(h) for h in mint_hashes]            │
│                                                                             │
│  3. BUILD INSTRUCTIONS                                                      │
│     Program-owned (PDAs + Tokens) → 1 decompress_idempotent ix              │
│     ATAs                          → N create_ata + 1 Transfer2 ix           │
│     Mints                         → N decompress_mint ix                    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Program Side: Macro-Generated Code

### From `#[derive(RentFreeAccount)]` on state structs:
- `HasCompressionInfo` impl
- `Pack`/`Unpack` impls (generates `PackedXxx` struct)
- `DataHasher` impl

### From `#[rentfree_program]` on module:
- `RentFreeAccountVariant` enum
- `TokenAccountVariant` enum  
- `XxxSeeds` structs + `IntoVariant` impls
- `DecompressContext` impl

---

## Example: csdk-anchor-full-derived-test

### State (state.rs)

```rust
#[derive(Default, Debug, InitSpace, RentFreeAccount)]
#[account]
pub struct UserRecord {
    pub compression_info: Option<CompressionInfo>,
    pub owner: Pubkey,
    #[max_len(32)]
    pub name: String,
    pub score: u64,
    pub category_id: u64,
}

#[derive(Default, Debug, InitSpace, RentFreeAccount)]
#[account]
pub struct GameSession {
    pub compression_info: Option<CompressionInfo>,
    pub session_id: u64,
    pub player: Pubkey,
    #[max_len(32)]
    pub game_type: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub score: u64,
}
```

### Instruction Accounts (instruction_accounts.rs)

```rust
#[derive(Accounts, RentFree)]
#[instruction(params: FullAutoWithMintParams)]
pub struct CreatePdasAndMintAuto<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    pub authority: Signer<'info>,

    #[account(
        init,
        seeds = [b"user_record", authority.key().as_ref(), params.owner.as_ref(), ...],
        bump,
    )]
    #[rentfree]
    pub user_record: Account<'info, UserRecord>,

    #[account(
        init,
        seeds = [b"game_session", max_key(&fee_payer.key(), &authority.key()).as_ref(), ...],
        bump,
    )]
    #[rentfree]
    pub game_session: Account<'info, GameSession>,

    #[light_mint(mint_signer = mint_signer, authority = mint_authority, decimals = 9)]
    pub cmint: UncheckedAccount<'info>,

    #[account(mut, seeds = [VAULT_SEED, cmint.key().as_ref()], bump)]
    #[rentfree_token(authority = [b"vault_authority"])]
    pub vault: UncheckedAccount<'info>,
}
```

### Generated Types

```rust
// RentFreeAccountVariant - all compressible types
pub enum RentFreeAccountVariant {
    // PDAs (unpacked with ctx.* seed pubkeys)
    UserRecord { data: UserRecord, authority: Pubkey, mint_authority: Pubkey },
    GameSession { data: GameSession, fee_payer: Pubkey, authority: Pubkey },
    
    // PDAs (packed with indices into remaining_accounts)
    PackedUserRecord { data: PackedUserRecord, authority_idx: u8, mint_authority_idx: u8 },
    PackedGameSession { data: PackedGameSession, fee_payer_idx: u8, authority_idx: u8 },
    
    // Program-owned tokens
    PackedCTokenData(PackedCTokenData<PackedTokenAccountVariant>),
    CTokenData(CTokenData<TokenAccountVariant>),
}

// TokenAccountVariant - program-owned token accounts
// Captures ctx.* seeds needed for token account + authority derivation
pub enum TokenAccountVariant {
    Vault { cmint: Pubkey },  // Token seeds: [VAULT_SEED, cmint]
                              // Authority seeds: [b"vault_authority"] (static, no ctx.*)
}

pub enum PackedTokenAccountVariant {
    Vault { cmint_idx: u8 },
}

// Authority seeds from #[rentfree_token(authority = [b"vault_authority"])]
// Used during decompress to verify/derive the authority PDA that owns the token account

// Seeds structs - for client variant construction
pub struct UserRecordSeeds {
    pub authority: Pubkey,
    pub mint_authority: Pubkey,
    pub owner: Pubkey,
    pub category_id: u64,
}

impl IntoVariant<RentFreeAccountVariant> for UserRecordSeeds {
    fn into_variant(self, data: &[u8]) -> Result<RentFreeAccountVariant, Error> {
        let user_record = UserRecord::try_from_slice(data)?;
        // Verify data.* seeds match
        Ok(RentFreeAccountVariant::UserRecord {
            data: user_record,
            authority: self.authority,
            mint_authority: self.mint_authority,
        })
    }
}
```

---

## NEW: SDK Additions to Macro

```rust
// Add to generated code:
pub struct SeedContext {
    pub authority: Option<Pubkey>,
    pub mint_authority: Option<Pubkey>,
    pub fee_payer: Option<Pubkey>,
    // All ctx.* fields extracted from seeds
}

impl RentFreeAccountVariant {
    pub fn from_parsed(
        data: &[u8],
        discriminator: &[u8; 8],
        ctx: &SeedContext,
    ) -> Result<Self, ProgramError> {
        match discriminator {
            x if x == UserRecord::LIGHT_DISCRIMINATOR => {
                let parsed = UserRecord::try_from_slice(&data[8..])?;
                Ok(Self::UserRecord {
                    data: parsed,
                    authority: ctx.authority.unwrap(),
                    mint_authority: ctx.mint_authority.unwrap(),
                })
            }
            // ... other variants
        }
    }
}
```

---

## SDK Implementation (per program)

```rust
// =============================================================================
// FULL IMPLEMENTATION: raydium-cp-swap
// =============================================================================
//
// DESIGN: All seed values extracted from PoolState fields.
// PoolState stores: token_0_vault, token_1_vault, lp_mint, token_0_mint, etc.
// =============================================================================

use std::collections::HashMap;
use anchor_lang::prelude::*;
use light_sdk::LightDiscriminator;
use light_token_sdk::token::find_mint_address;

use raydium_cp_swap::{
    PoolState, ObservationState,
    RentFreeAccountVariant, TokenAccountVariant,
    POOL_SEED, POOL_VAULT_SEED, OBSERVATION_SEED, AUTH_SEED,
    ID as PROGRAM_ID,
};
use raydium_cp_swap::instructions::initialize::LP_MINT_SIGNER_SEED;

// -----------------------------------------------------------------------------
// OPERATIONS
// -----------------------------------------------------------------------------

pub enum Operation {
    Swap,
    Deposit,
    Withdraw,
}

// -----------------------------------------------------------------------------
// SEED ANALYSIS (from initialize.rs)
// -----------------------------------------------------------------------------
//
// COMPRESSIBLE ACCOUNTS:
//
// 1. pool_state - #[rentfree]
//    seeds: [POOL_SEED, amm_config, token_0_mint, token_1_mint]
//    
// 2. token_0_vault - #[rentfree_token(authority = [AUTH_SEED])]
//    seeds: [POOL_VAULT_SEED, pool_state, token_0_mint]
//    
// 3. token_1_vault - #[rentfree_token(authority = [AUTH_SEED])]
//    seeds: [POOL_VAULT_SEED, pool_state, token_1_mint]
//    
// 4. observation_state - #[rentfree]
//    seeds: [OBSERVATION_SEED, pool_state]
//    
// 5. lp_mint - #[light_mint]
//    derived from lp_mint_signer = PDA([LP_MINT_SIGNER_SEED, pool_state])
//
// KEY INSIGHT: PoolState stores vault pubkeys directly!
//   - token_0_vault: Pubkey
//   - token_1_vault: Pubkey  
//   - lp_mint: Pubkey
//   - token_0_mint: Pubkey
//   - token_1_mint: Pubkey
//   - amm_config: Pubkey
//   - observation_key: Pubkey
// -----------------------------------------------------------------------------

// -----------------------------------------------------------------------------
// SDK STRUCT
// -----------------------------------------------------------------------------

pub struct RaydiumCpSwapSdk {
    // === EXTRACTED FROM POOLSTATE ===
    pool_state_pubkey: Option<Pubkey>,
    amm_config: Option<Pubkey>,
    token_0_mint: Option<Pubkey>,
    token_1_mint: Option<Pubkey>,
    token_0_vault: Option<Pubkey>,   // Stored directly in PoolState!
    token_1_vault: Option<Pubkey>,   // Stored directly in PoolState!
    lp_mint: Option<Pubkey>,         // Stored directly in PoolState!
    observation_key: Option<Pubkey>, // Stored directly in PoolState!
    
    // === DERIVED ===
    authority: Option<Pubkey>,       // PDA([AUTH_SEED])
    lp_mint_signer: Option<Pubkey>,  // PDA([LP_MINT_SIGNER_SEED, pool_state])
    
    // === SPECS CACHE ===
    program_owned_specs: HashMap<Pubkey, ProgramOwnedSpec<RentFreeAccountVariant>>,
    ata_specs: HashMap<Pubkey, AtaSpec>,
    mint_specs: HashMap<Pubkey, MintSpec>,
}

impl Default for RaydiumCpSwapSdk {
    fn default() -> Self {
        Self {
            pool_state_pubkey: None,
            amm_config: None,
            token_0_mint: None,
            token_1_mint: None,
            token_0_vault: None,
            token_1_vault: None,
            lp_mint: None,
            observation_key: None,
            authority: None,
            lp_mint_signer: None,
            program_owned_specs: HashMap::new(),
            ata_specs: HashMap::new(),
            mint_specs: HashMap::new(),
        }
    }
}

// -----------------------------------------------------------------------------
// CORE: PARSING POOLSTATE → EXTRACTING ALL FIELDS
// -----------------------------------------------------------------------------

impl RaydiumCpSwapSdk {
    fn parse_pool_state(&mut self, account: &KeyedAccountInterface) -> Result<()> {
        let pool = PoolState::try_from_slice(&account.data[8..])?;
        
        // Store pool pubkey
        self.pool_state_pubkey = Some(account.pubkey);
        
        // Extract ALL pubkeys directly from PoolState fields
        self.amm_config = Some(pool.amm_config);
        self.token_0_mint = Some(pool.token_0_mint);
        self.token_1_mint = Some(pool.token_1_mint);
        self.token_0_vault = Some(pool.token_0_vault);  // Directly stored!
        self.token_1_vault = Some(pool.token_1_vault);  // Directly stored!
        self.lp_mint = Some(pool.lp_mint);              // Directly stored!
        self.observation_key = Some(pool.observation_key);
        
        // Derive authority PDA
        let (authority, _) = Pubkey::find_program_address(
            &[AUTH_SEED.as_bytes()],
            &PROGRAM_ID,
        );
        self.authority = Some(authority);
        
        // Derive lp_mint_signer PDA
        let (lp_mint_signer, _) = Pubkey::find_program_address(
            &[LP_MINT_SIGNER_SEED, account.pubkey.as_ref()],
            &PROGRAM_ID,
        );
        self.lp_mint_signer = Some(lp_mint_signer);
        
        // Build PoolState spec
        let variant = RentFreeAccountVariant::PoolState {
            data: pool,
            amm_config: self.amm_config.unwrap(),
            token_0_mint: self.token_0_mint.unwrap(),
            token_1_mint: self.token_1_mint.unwrap(),
        };
        
        self.program_owned_specs.insert(account.pubkey, ProgramOwnedSpec {
            address: account.pubkey,
            variant,
            is_cold: account.is_cold,
            cold_context: account.cold_context.clone(),
        });
        
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// TRAIT IMPLEMENTATION
// -----------------------------------------------------------------------------

impl CompressibleProgram for RaydiumCpSwapSdk {
    type Variant = RentFreeAccountVariant;

    fn from_keyed_accounts(accounts: &[KeyedAccountInterface]) -> Result<Self> {
        let mut sdk = Self::default();
        
        for account in accounts {
            if account.data.len() < 8 { continue; }
            let disc: [u8; 8] = account.data[..8].try_into()?;
            
            if disc == PoolState::LIGHT_DISCRIMINATOR {
                sdk.parse_pool_state(account)?;
            } else {
                sdk.parse_account(account)?;
            }
        }
        
        Ok(sdk)
    }

    fn get_accounts_to_update(&self, op: Operation) -> Vec<Pubkey> {
        match op {
            Operation::Swap => {
                // Swap needs: vaults (for balance check)
                vec![
                    self.token_0_vault,
                    self.token_1_vault,
                ].into_iter().flatten().collect()
            }
            Operation::Deposit | Operation::Withdraw => {
                // Deposit/Withdraw needs: vaults + lp_mint
                vec![
                    self.token_0_vault,
                    self.token_1_vault,
                    self.lp_mint,
                ].into_iter().flatten().collect()
            }
        }
    }

    fn update(&mut self, accounts: &[KeyedAccountInterface]) -> Result<()> {
        for account in accounts {
            self.parse_account(account)?;
        }
        Ok(())
    }

    fn get_all_specs(&self) -> AllSpecs<Self::Variant> {
        AllSpecs {
            program_owned: self.program_owned_specs.values().cloned().collect(),
            atas: self.ata_specs.values().cloned().collect(),
            mints: self.mint_specs.values().cloned().collect(),
        }
    }

    fn get_specs_for_operation(&self, op: Operation) -> AllSpecs<Self::Variant> {
        let keys: Vec<Pubkey> = match op {
            Operation::Swap => vec![
                self.pool_state_pubkey,
                self.token_0_vault,
                self.token_1_vault,
            ],
            Operation::Deposit | Operation::Withdraw => vec![
                self.pool_state_pubkey,
                self.token_0_vault,
                self.token_1_vault,
                self.lp_mint,
            ],
        }.into_iter().flatten().collect();
        
        AllSpecs {
            program_owned: keys.iter()
                .filter_map(|k| self.program_owned_specs.get(k).cloned())
                .collect(),
            atas: self.ata_specs.values().cloned().collect(),
            mints: keys.iter()
                .filter_map(|k| self.mint_specs.get(k).cloned())
                .collect(),
        }
    }
}

// -----------------------------------------------------------------------------
// ACCOUNT PARSING
// -----------------------------------------------------------------------------

impl RaydiumCpSwapSdk {
    fn parse_account(&mut self, account: &KeyedAccountInterface) -> Result<()> {
        if account.data.len() < 8 { return Ok(()); }
        let disc: [u8; 8] = account.data[..8].try_into()?;
        
        if disc == ObservationState::LIGHT_DISCRIMINATOR {
            self.parse_observation(account)?;
        } else if Some(account.pubkey) == self.token_0_vault {
            self.parse_vault_0(account)?;
        } else if Some(account.pubkey) == self.token_1_vault {
            self.parse_vault_1(account)?;
        } else if Some(account.pubkey) == self.lp_mint {
            self.parse_lp_mint(account)?;
        }
        Ok(())
    }
    
    fn parse_observation(&mut self, account: &KeyedAccountInterface) -> Result<()> {
        let data = ObservationState::try_from_slice(&account.data[8..])?;
        
        // ObservationState seeds: [OBSERVATION_SEED, pool_state]
        // pool_state is ctx.* seed - extracted from self
        let variant = RentFreeAccountVariant::ObservationState {
            data,
            pool_state: self.pool_state_pubkey.ok_or(Error::msg("parse pool first"))?,
        };
        
        self.program_owned_specs.insert(account.pubkey, ProgramOwnedSpec {
            address: account.pubkey,
            variant,
            is_cold: account.is_cold,
            cold_context: account.cold_context.clone(),
        });
        Ok(())
    }
    
    fn parse_vault_0(&mut self, account: &KeyedAccountInterface) -> Result<()> {
        let token_data = parse_token_data(&account.data)?;
        
        // Vault0 seeds: [POOL_VAULT_SEED, pool_state, token_0_mint]
        // Authority seeds: [AUTH_SEED]
        let variant = RentFreeAccountVariant::CTokenData(CTokenData {
            variant: TokenAccountVariant::Vault0 {
                pool_state: self.pool_state_pubkey.ok_or(Error::msg("parse pool first"))?,
                token_0_mint: self.token_0_mint.ok_or(Error::msg("parse pool first"))?,
            },
            token_data,
        });
        
        self.program_owned_specs.insert(account.pubkey, ProgramOwnedSpec {
            address: account.pubkey,
            variant,
            is_cold: account.is_cold,
            cold_context: account.cold_context.clone(),
        });
        Ok(())
    }
    
    fn parse_vault_1(&mut self, account: &KeyedAccountInterface) -> Result<()> {
        let token_data = parse_token_data(&account.data)?;
        
        let variant = RentFreeAccountVariant::CTokenData(CTokenData {
            variant: TokenAccountVariant::Vault1 {
                pool_state: self.pool_state_pubkey.ok_or(Error::msg("parse pool first"))?,
                token_1_mint: self.token_1_mint.ok_or(Error::msg("parse pool first"))?,
            },
            token_data,
        });
        
        self.program_owned_specs.insert(account.pubkey, ProgramOwnedSpec {
            address: account.pubkey,
            variant,
            is_cold: account.is_cold,
            cold_context: account.cold_context.clone(),
        });
        Ok(())
    }
    
    fn parse_lp_mint(&mut self, account: &KeyedAccountInterface) -> Result<()> {
        self.mint_specs.insert(account.pubkey, MintSpec {
            cmint: account.pubkey,
            mint_signer: self.lp_mint_signer.ok_or(Error::msg("parse pool first"))?,
            compressed_address: account.cold_context.as_ref()
                .map(|c| c.compressed_address).unwrap_or_default(),
            is_cold: account.is_cold,
            cold_context: account.cold_context.clone(),
        });
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// CLIENT USAGE
// -----------------------------------------------------------------------------
//
// // 1. Fetch pool state
// let pool = fetch_interface(&pool_state_pubkey).await?;
//
// // 2. Create SDK - parses PoolState, extracts ALL pubkeys from fields
// let mut sdk = RaydiumCpSwapSdk::from_keyed_accounts(&[pool])?;
//
// // 3. Get accounts for Swap - vault pubkeys come from PoolState fields!
// let to_fetch = sdk.get_accounts_to_update(Operation::Swap);
// // Returns: [token_0_vault, token_1_vault] - no derivation needed, stored in PoolState
//
// // 4. Fetch and update
// let interfaces = fetch_interfaces(&to_fetch).await?;
// sdk.update(&interfaces)?;
//
// // 5. Build load instructions
// let specs = sdk.get_specs_for_operation(Operation::Swap);
// let load_ixs = build_load_instructions(&specs, &config).await?;
```

---

## Client Flows

### Simple Client (one-off transaction)

```rust
// Knows all accounts upfront, fetches everything
let interfaces = fetch_interfaces(&[pool, vault_0, vault_1, ata, mint]).await?;
let ctx = RaydiumSdk::from_keyed_accounts(&interfaces)?;
let specs = ctx.get_all_specs();
let load_ixs = build_load_instructions(&specs, &config).await?;
```

### Aggregator (cached, operation-aware)

```rust
// 1. Initialize from canonical root(s) only
let pool_interface = fetch_interface(&pool_pubkey).await?;
let mut ctx = RaydiumSdk::from_keyed_accounts(&[pool_interface])?;

// 2. Discover what else to fetch for Swap operation
let needed = ctx.get_accounts_to_update(Operation::Swap);  // → [vault_0, vault_1]

// 3. Fetch and update cache
let more_interfaces = fetch_interfaces(&needed).await?;
ctx.update(&more_interfaces)?;  // Parses, builds specs, updates internal cache

// 4. Get specs filtered for operation
let specs = ctx.get_specs_for_operation(Operation::Swap);

// 5. Build load instructions
let load_ixs = build_load_instructions(&specs, &config).await?;

// --- Later, cache refresh ---
let refresh_keys = ctx.get_accounts_to_update(Operation::Swap);
let fresh = fetch_interfaces(&refresh_keys).await?;
ctx.update(&fresh)?;  // Updates is_cold flags, etc.
```

### Aggregator Dispatch (multiple programs)

```rust
match pool_type {
    Raydium => {
        let mut ctx = RaydiumSdk::from_keyed_accounts(&[pool_iface])?;
        let needed = ctx.get_accounts_to_update(Operation::Swap);
        ctx.update(&fetch_interfaces(&needed).await?)?;
        build_load_instructions(&ctx.get_specs_for_operation(Operation::Swap), &cfg).await
    }
    Orca => {
        let mut ctx = OrcaSdk::from_keyed_accounts(&[pool_iface])?;
        let needed = ctx.get_accounts_to_update(Operation::Swap);
        ctx.update(&fetch_interfaces(&needed).await?)?;
        build_load_instructions(&ctx.get_specs_for_operation(Operation::Swap), &cfg).await
    }
}
```

---

## Full Trait

```rust
pub trait CompressibleProgram {
    type Variant: Pack + Clone + std::fmt::Debug;

    /// Construct from canonical root(s). Parses, extracts SeedContext.
    fn from_keyed_accounts(accounts: &[KeyedAccountInterface]) -> Result<Self> where Self: Sized;
    
    /// Returns pubkeys needed for operation (derived from root state).
    fn get_accounts_to_update(&self, op: Operation) -> Vec<Pubkey>;
    
    /// Update internal cache with new account data. Idempotent.
    fn update(&mut self, accounts: &[KeyedAccountInterface]) -> Result<()>;
    
    /// All specs (for simple clients who fetch everything).
    fn get_all_specs(&self) -> AllSpecs<Self::Variant>;
    
    /// Specs filtered by operation (for aggregators).
    fn get_specs_for_operation(&self, op: Operation) -> AllSpecs<Self::Variant>;
}

pub enum Operation {
    Swap,
    Deposit,
    Withdraw,
    // Program-specific
}
```

---

## System Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              PROGRAM                                         │
│  #[rentfree_program] + #[derive(RentFreeAccount)]                           │
│                          ↓                                                   │
│  Generated: RentFreeAccountVariant, TokenAccountVariant, XxxSeeds,          │
│             IntoVariant, Pack/Unpack, SeedContext, from_parsed()            │
└────────────────────────────────────┬────────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                            PROGRAM SDK                                       │
│  impl CompressibleProgram for MyProgramSdk {                                │
│      type Variant = RentFreeAccountVariant;                                 │
│      fn from_keyed_accounts([root]) → parse root, extract SeedContext       │
│      fn get_accounts_to_update(op) → derived pubkeys for operation          │
│      fn update(interfaces) → parse, build specs, cache                      │
│      fn get_specs_for_operation(op) → filtered AllSpecs<V>                  │
│  }                                                                          │
└────────────────────────────────────┬────────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         AGGREGATOR FLOW                                      │
│                                                                             │
│  1. from_keyed_accounts([pool])     // Init from root                       │
│  2. get_accounts_to_update(Swap)    // What to fetch                        │
│  3. update(fetched_interfaces)      // Fill cache                           │
│  4. get_specs_for_operation(Swap)   // Get relevant specs                   │
│  5. build_load_instructions(specs)  // Build decompress ixs                 │
│                                                                             │
│  Cache refresh: repeat 2-5                                                  │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## State Change Diagram: Client <> CompressibleProgram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    SDK INTERNAL STATE TRANSITIONS                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  STATE 0: Uninitialized                                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  pool_pubkey: None                                                   │   │
│  │  seed_context: Empty                                                 │   │
│  │  specs: {}                                                           │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                          │                                                  │
│                          │ from_keyed_accounts([pool_iface])                │
│                          ▼                                                  │
│  STATE 1: Root Parsed (seeds extracted, addresses derived)                  │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  pool_pubkey: Some(0xABC...)                                         │   │
│  │  seed_context: { token_0_mint, token_1_mint, amm_config, ... }       │   │
│  │  derived: { vault_0: 0xDEF, vault_1: 0x123, lp_mint: 0x456 }         │   │
│  │  specs: { pool_state: Spec { filled: true, is_cold: ? } }            │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                          │                                                  │
│                          │ get_accounts_to_update(Swap)                     │
│                          │ → returns [vault_0, vault_1]                     │
│                          │                                                  │
│                          │ update([vault_0_iface, vault_1_iface])           │
│                          ▼                                                  │
│  STATE 2: Operation Ready (all specs for op filled)                         │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  specs: {                                                            │   │
│  │    pool_state: Spec { filled: true, is_cold: false }                 │   │
│  │    vault_0:    Spec { filled: true, is_cold: true  }  ← cold!        │   │
│  │    vault_1:    Spec { filled: true, is_cold: false }                 │   │
│  │  }                                                                   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                          │                                                  │
│                          │ get_specs_for_operation(Swap)                    │
│                          │ → Ok(AllSpecs { ... })                           │
│                          ▼                                                  │
│  STATE 3: Specs Returned → Client calls build_load_instructions()           │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                    CLIENT <> TRAIT INTERACTION FLOW                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  CLIENT                              SDK (CompressibleProgram)              │
│  ──────                              ────────────────────────               │
│                                                                             │
│  ┌──────────────────┐                                                       │
│  │ Know pool pubkey │                                                       │
│  └────────┬─────────┘                                                       │
│           │                                                                 │
│           │ fetch pool_iface from RPC/indexer                               │
│           ▼                                                                 │
│  ┌──────────────────┐     from_keyed_accounts([pool])                       │
│  │ Have pool data   │ ─────────────────────────────────►  Parse pool        │
│  └────────┬─────────┘                                     Extract seeds     │
│           │                                               Derive addresses  │
│           │                                               Store root spec   │
│           │                                                     │           │
│           │◄─────────────────────────────────────────────── Ok(sdk)         │
│           │                                                                 │
│           │              get_accounts_to_update(Swap)                       │
│           │ ─────────────────────────────────────────────►  Check op reqs   │
│           │                                                 Return pubkeys  │
│           │◄─────────────────────────────────────────────── [v0, v1]        │
│           │                                                                 │
│           │ fetch v0_iface, v1_iface from RPC/indexer                       │
│           ▼                                                                 │
│  ┌──────────────────┐     update([v0, v1])                                  │
│  │ Have vault data  │ ─────────────────────────────────►  Parse accounts    │
│  └────────┬─────────┘                                     Build variants    │
│           │                                               Set is_cold flags │
│           │                                               Cache specs       │
│           │◄─────────────────────────────────────────────── Ok(())          │
│           │                                                                 │
│           │              get_specs_for_operation(Swap)                      │
│           │ ─────────────────────────────────────────────►  Validate filled │
│           │                                                 Filter by op    │
│           │◄─── Ok(AllSpecs) or Err(IncompleteContext) ───                  │
│           │                                                                 │
│           │ (if Ok)                                                         │
│           │ build_load_instructions(specs, config)                          │
│           ▼                                                                 │
│  ┌──────────────────┐                                                       │
│  │ Have load ixs    │                                                       │
│  │ Execute tx       │                                                       │
│  └──────────────────┘                                                       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                         ERROR HANDLING FLOW                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  get_specs_for_operation(Deposit)                                           │
│           │                                                                 │
│           ▼                                                                 │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Check: Deposit needs [pool, vault_0, vault_1, lp_mint]              │   │
│  │        specs has:    [pool, vault_0, vault_1]                       │   │
│  │        missing:      [lp_mint]                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│           │                                                                 │
│           ▼                                                                 │
│  Err(IncompleteContext::MissingAccount(lp_mint_pubkey))                     │
│           │                                                                 │
│           │ Client handles:                                                 │
│           ▼                                                                 │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ 1. Fetch lp_mint_iface                                              │   │
│  │ 2. sdk.update([lp_mint_iface])                                      │   │
│  │ 3. Retry get_specs_for_operation(Deposit) → Ok(AllSpecs)            │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                     AGGREGATOR CACHE REFRESH CYCLE                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  TIME T0: Initial setup                                                     │
│  ─────────────────────                                                      │
│  pools: HashMap<Pubkey, Sdk> = {                                            │
│      0xABC: Sdk { state: OperationReady, specs: {...} }                     │
│      0xDEF: Sdk { state: OperationReady, specs: {...} }                     │
│  }                                                                          │
│                                                                             │
│  TIME T1: Refresh cycle (every N slots)                                     │
│  ───────────────────────────────────────                                    │
│  for (pool_key, sdk) in pools.iter_mut() {                                  │
│      let to_refresh = sdk.get_accounts_to_update(Swap);                     │
│      let fresh = fetch_interfaces(&to_refresh).await;                       │
│      sdk.update(&fresh)?;  // Updates is_cold flags, balances, etc.         │
│  }                                                                          │
│                                                                             │
│  TIME T2: Swap request for pool 0xABC                                       │
│  ─────────────────────────────────────                                      │
│  let sdk = pools.get(&0xABC)?;                                              │
│  let specs = sdk.get_specs_for_operation(Swap)?;  // Already filled!        │
│  let ixs = build_load_instructions(&specs, &cfg).await?;                    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```
