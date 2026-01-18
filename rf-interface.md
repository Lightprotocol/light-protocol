# RentFree Interface Trait Implementation

Client-side SDK pattern for programs with compressible (rent-free) accounts.

## Overview

This implementation provides a trait-based approach for programs to expose their compressible account structure to clients. Inspired by Jupiter's AMM interface pattern.

**Core idea**: Programs implement `CompressibleProgram` trait in a client-side SDK module. Clients use this SDK to discover accounts, build specs, and generate decompression instructions.

## Architecture

```
                     CLIENT FLOW
    ================================================
    
    [1] Fetch root account (e.g., PoolState)
              |
              v
    [2] AmmSdk::from_keyed_accounts([pool])
              |
              +-- parses PoolState
              +-- extracts pubkeys (vaults, mints, etc.)
              +-- derives PDAs (authority, mint_signer)
              +-- builds initial spec for pool_state
              |
              v
    [3] sdk.get_accounts_to_update(&Operation)
              |
              +-- returns pubkeys + types needed
              |
              v
    [4] Client fetches accounts by type:
              +-- PDAs: get_account_info_interface()
              +-- Tokens: get_token_account_interface()
              +-- Mints: get_mint_interface()
              |
              v
    [5] sdk.update(&keyed_accounts)
              |
              +-- parses each account
              +-- builds variant with seed values
              +-- caches spec (hot or cold)
              |
              v
    [6] sdk.get_specs_for_operation(&Operation)
              |
              +-- returns AllSpecs with program_owned, atas, mints
              |
              v
    [7] create_load_instructions_from_specs(&specs, ...)
              |
              +-- filters cold accounts
              +-- fetches proofs
              +-- builds decompress instructions
              |
              v
    [8] Execute transactions
```

## Sequence Diagram: RPC -> Client -> AmmSdk (Pure Sync)

**Key Principle**: SDK is 100% synchronous. Client does ALL RPC calls. SDK only processes pre-fetched data.

```
  RPC / INDEXER                      CLIENT                           AmmSdk (trait impl)
  =============                      ======                           ===================
       |                               |                              [PURE SYNC - NO I/O]
       |                               |                                     |
       |                               |                                     |
  [1] BOOTSTRAP: Fetch root state      |                                     |
       |                               |                                     |
       |<--- get_account_info_interface(pool_pubkey, program_id) ------------|
       |     [async RPC]               |                                     |
       |                               |                                     |
       |--- AccountInfoInterface ----->|                                     |
       |    { pubkey, is_cold,         |                                     |
       |      data, load_context }     |                                     |
       |                               |                                     |
       |                               |  keyed = KeyedAccountInterface      |
       |                               |    ::from_pda_interface(pool)       |
       |                               |  [local struct conversion]          |
       |                               |                                     |
       |                               |                                     |
  [2] INIT SDK                         |                                     |
       |                               |                                     |
       |                               |--- from_keyed_accounts(&[keyed]) -->|
       |                               |    [SYNC CALL]                      |
       |                               |                                     |
       |                               |                     +--[ SYNC ]---+ |
       |                               |                     | deserialize | |
       |                               |                     |  PoolState  | |
       |                               |                     | extract:    | |
       |                               |                     |  vaults,    | |
       |                               |                     |  mints,     | |
       |                               |                     |  obs_key    | |
       |                               |                     | derive PDAs | |
       |                               |                     | cache spec  | |
       |                               |                     +-------------+ |
       |                               |                                     |
       |                               |<--------- Ok(AmmSdk) [sync] --------|
       |                               |                                     |
       |                               |                                     |
  [3] DISCOVER: What accounts needed?  |                                     |
       |                               |                                     |
       |                               |--- get_accounts_to_update_typed --->|
       |                               |    (&AmmOperation::Deposit)         |
       |                               |    [SYNC CALL]                      |
       |                               |                                     |
       |                               |                     +--[ SYNC ]---+ |
       |                               |                     | lookup from | |
       |                               |                     | cached pks  | |
       |                               |                     +-------------+ |
       |                               |                                     |
       |                               |<-- Vec<AccountToFetch> [sync] ------|
       |                               |    [                                |
       |                               |      (vault_0, TokenAccount),       |
       |                               |      (vault_1, TokenAccount),       |
       |                               |      (observation, Pda),            |
       |                               |    ]                                |
       |                               |                                     |
       |                               |                                     |
  [4] FETCH: Client fetches each       |                                     |
       |                               |                                     |
       |<--- get_token_account_interface(vault_0) ---------------------------|
       |     [async RPC]               |                                     |
       |--- TokenAccountInterface ---->|                                     |
       |                               |                                     |
       |<--- get_token_account_interface(vault_1) ---------------------------|
       |     [async RPC]               |                                     |
       |--- TokenAccountInterface ---->|                                     |
       |                               |                                     |
       |<--- get_account_info_interface(observation) ------------------------|
       |     [async RPC]               |                                     |
       |--- AccountInfoInterface ----->|                                     |
       |                               |                                     |
       |                               |  keyed_accounts = interfaces        |
       |                               |    .map(KeyedAccountInterface::from)|
       |                               |  [local conversions]                |
       |                               |                                     |
       |                               |                                     |
  [5] UPDATE: Feed fetched data to SDK |                                     |
       |                               |                                     |
       |                               |--- sdk.update(&keyed_accounts) ---->|
       |                               |    [SYNC CALL]                      |
       |                               |                                     |
       |                               |                     +--[ SYNC ]---+ |
       |                               |                     | for each:   | |
       |                               |                     |  match pk   | |
       |                               |                     |  parse data | |
       |                               |                     |  build var  | |
       |                               |                     |  cache spec | |
       |                               |                     +-------------+ |
       |                               |                                     |
       |                               |<---------- Ok(()) [sync] -----------|
       |                               |                                     |
       |                               |                                     |
  [6] GET SPECS                        |                                     |
       |                               |                                     |
       |                               |--- get_specs_for_operation -------->|
       |                               |    (&AmmOperation::Deposit)         |
       |                               |    [SYNC CALL]                      |
       |                               |                                     |
       |                               |                     +--[ SYNC ]---+ |
       |                               |                     | filter by   | |
       |                               |                     | operation   | |
       |                               |                     +-------------+ |
       |                               |                                     |
       |                               |<------- AllSpecs { [sync] ----------|
       |                               |           program_owned: [...],     |
       |                               |           atas: [],                 |
       |                               |           mints: [...],             |
       |                               |         }                           |
       |                               |                                     |
       |                               |                                     |
  [7] BUILD INSTRUCTIONS (if cold)     |                                     |
       |                               |                                     |
       |                               |  if specs.has_cold():               |
       |                               |                                     |
       |                               |  hashes = specs.program_owned       |
       |                               |    .filter(|s| s.is_cold)           |
       |                               |    .map(|s| s.cold_context          |
       |                               |      .compressed_account.hash)      |
       |                               |  [local extraction from specs]      |
       |                               |                                     |
       |<--- get_validity_proof(hashes) -------------------------------------|
       |     [async RPC]               |                                     |
       |--- ValidityProofWithContext ->|                                     |
       |                               |                                     |
       |                               |  ixs = build_decompress_ixs(        |
       |                               |    specs, proof)                    |
       |                               |  [local instruction building]       |
       |                               |                                     |
       |                               |                                     |
  [8] EXECUTE                          |                                     |
       |                               |                                     |
       |<--- send_transaction(ixs) ------------------------------------------|
       |     [async RPC]               |                                     |
       |--- confirmed ---------------->|                                     |
       |                               |                                     |
       v                               v                                     v


  TRAIT METHODS (all sync)
  ========================

  impl CompressibleProgram for AmmSdk {
      type Variant = RentFreeAccountVariant;
      type Operation = AmmOperation;
      type Error = AmmSdkError;

      fn from_keyed_accounts(&[KeyedAccountInterface]) -> Result<Self>
      fn get_accounts_to_update(&self, &Operation) -> Vec<Pubkey>
      fn update(&mut self, &[KeyedAccountInterface]) -> Result<()>
      fn get_all_specs(&self) -> AllSpecs<Variant>
      fn get_specs_for_operation(&self, &Operation) -> AllSpecs<Variant>
  }

  EXTENSION METHOD (also sync)
  ============================

  impl AmmSdk {
      fn get_accounts_to_update_typed(&self, &Operation) -> Vec<AccountToFetch>
  }
```

## Component Diagram: RPC -> Client -> SDK

```
+-------------------+         +-------------------------+         +----------------------+
|                   |         |                         |         |                      |
|   RPC / INDEXER   |         |        CLIENT           |         |    AmmSdk            |
|                   |         |      (async I/O)        |         |  (CompressibleProgram|
|                   |         |                         |         |   trait impl)        |
+-------------------+         +-------------------------+         +----------------------+
         |                              |                                   |
         |                              |                                   |
         |    ALL ASYNC I/O             |     ALL SYNC CALLS                |
         |    <===============          |     ===============>              |
         |                              |                                   |
         |                              |                                   |
         |  getAccountInfo              |                                   |
         |  getCompressedAccount        |  from_keyed_accounts()            |
         |  getCompressedTokenAccounts  |  get_accounts_to_update()         |
         |  getValidityProof            |  update()                         |
         |  sendTransaction             |  get_specs_for_operation()        |
         |                              |                                   |
         |                              |                                   |
         v                              v                                   v


    DATA FLOW
    =========

    RPC --(AccountInfoInterface)--> Client --(KeyedAccountInterface)--> SDK
                                       |                                  |
                                       |                                  |
                                       |<-----(Vec<Pubkey>)---------------+
                                       |       what to fetch next
                                       |
    RPC <--(fetch by pubkey)---------- |
                                       |
                                       |
                                       |<-----(AllSpecs)------------------+
                                       |       specs with cold_context
                                       |
    RPC <--(get_validity_proof)------- |
                                       |
                                       |  build_decompress_ixs() [local]
                                       |
    RPC <--(send_transaction)--------- |
```

## Responsibility Matrix

```
+----------------------------------+-------------------+-------------------+
|           OPERATION              |      CLIENT       |      AmmSdk       |
+----------------------------------+-------------------+-------------------+
| Fetch account from RPC           |        X          |                   |
| Fetch token account from RPC     |        X          |                   |
| Fetch proof from indexer         |        X          |                   |
| Send transaction                 |        X          |                   |
| Network error handling           |        X          |                   |
+----------------------------------+-------------------+-------------------+
| Deserialize account data         |                   |        X          |
| Extract pubkeys from state       |                   |        X          |
| Derive PDAs deterministically    |                   |        X          |
| Build RentFreeAccountVariant     |                   |        X          |
| Cache specs internally           |                   |        X          |
| Filter specs by operation        |                   |        X          |
| Return what accounts to fetch    |                   |        X          |
+----------------------------------+-------------------+-------------------+
| Convert Interface -> Keyed       |        X          |                   |
| Extract hashes from specs        |        X          |                   |
| Build Instruction from specs     |        X          |                   |
+----------------------------------+-------------------+-------------------+

SDK Contract:
  - NO async
  - NO RPC calls
  - NO network I/O
  - Deterministic: same input -> same output
  - All methods return immediately (sync)
```

## Data Flow: Hot vs Cold Path

```
                    ACCOUNT STATE CHECK
                    ===================

    +-------------+
    | Account     |
    | Pubkey      |
    +------+------+
           |
           v
    +------+------+     YES     +-----------------+
    | On-chain?   +------------>| HOT PATH        |
    | (lamports>0)|             |                 |
    +------+------+             | - Read on-chain |
           | NO                 | - is_cold=false |
           v                    | - No proof      |
    +------+------+             |   needed        |
    | Compressed? |             +-----------------+
    | (indexer)   |
    +------+------+
           | YES
           v
    +------+------+
    | COLD PATH   |
    |             |
    | - Fetch     |
    |   compressed|
    | - is_cold=  |
    |   true      |
    | - Store     |
    |   context   |
    | - Need      |
    |   proof     |
    +-------------+


              DECOMPRESSION DECISION
              ======================

    +-------------+
    | AllSpecs    |
    +------+------+
           |
           v
    +------+------+     YES     +-----------------+
    | all_hot()?  +------------>| SKIP            |
    |             |             | No instructions |
    +------+------+             | needed          |
           | NO                 +-----------------+
           v
    +------+------+
    | DECOMPRESS  |
    |             |
    | 1. Collect  |
    |    hashes   |
    | 2. Fetch    |
    |    proofs   |
    | 3. Build    |
    |    ixs      |
    | 4. Execute  |
    +-------------+
```

## New Types

### `AccountToFetch`

Descriptor for fetching accounts. Pass to `rpc.get_multiple_account_interfaces()`.

```rust
pub enum AccountToFetch {
    /// PDA - uses get_account_info_interface(address, program_id)
    Pda { address: Pubkey, program_id: Pubkey },
    /// Token account - uses get_token_account_interface(address)
    Token { address: Pubkey },
    /// Mint - uses get_mint_interface(signer)
    Mint { signer: Pubkey },
}
```

Constructors: `AccountToFetch::pda(addr, prog)`, `AccountToFetch::token(addr)`, `AccountToFetch::mint(signer)`

### `KeyedAccountInterface`

Wrapper for account data with explicit pubkey and cold/hot context.

```rust
pub struct KeyedAccountInterface {
    pub pubkey: Pubkey,
    pub is_cold: bool,
    pub data: Vec<u8>,
    pub cold_context: Option<ColdContext>,
}

pub enum ColdContext {
    Pda(PdaDecompressionContext),
    Token(TokenLoadContext),
}
```

**Constructors:**
- `from_pda_interface(AccountInfoInterface)` - for PDA accounts
- `from_token_interface(TokenAccountInterface)` - for token accounts
- `hot(pubkey, data)` - manually create hot account
- `cold_pda(pubkey, data, compressed_account)` - manually create cold PDA

### `ProgramOwnedSpec<V>`

Spec for PDAs and program-owned token accounts.

```rust
pub struct ProgramOwnedSpec<V> {
    pub address: Pubkey,
    pub variant: V,           // RentFreeAccountVariant with seed values
    pub is_cold: bool,
    pub cold_context: Option<PdaDecompressionContext>,
}
```

### `AtaSpec`

Spec for Associated Token Accounts.

```rust
pub struct AtaSpec {
    pub address: Pubkey,
    pub wallet_owner: Pubkey,
    pub mint: Pubkey,
    pub is_cold: bool,
    pub load_context: Option<TokenLoadContext>,
}
```

### `MintSpec`

Spec for Light Mints.

```rust
pub struct MintSpec {
    pub cmint: Pubkey,
    pub mint_signer: Pubkey,
    pub compressed_address: [u8; 32],
    pub is_cold: bool,
    pub compressed: Option<CompressedAccount>,
    pub mint_data: Option<Mint>,  // Parsed mint data for cold mints
}
```

### `AllSpecs<V>`

Collection of all specs grouped by type.

```rust
pub struct AllSpecs<V> {
    pub program_owned: Vec<ProgramOwnedSpec<V>>,
    pub atas: Vec<AtaSpec>,
    pub mints: Vec<MintSpec>,
}
```

Helper methods:
- `all_hot()` - true if no decompression needed
- `has_cold()` - true if any account needs decompression
- `cold_program_owned()` / `cold_atas()` / `cold_mints()` - filtered iterators

## CompressibleProgram Trait

```rust
pub trait CompressibleProgram: Sized {
    type Variant: Pack + Clone + Debug;  // RentFreeAccountVariant
    type Operation;                       // Program-specific enum
    type Error: std::error::Error;

    fn from_keyed_accounts(accounts: &[KeyedAccountInterface]) -> Result<Self, Self::Error>;
    fn get_accounts_to_update(&self, op: &Self::Operation) -> Vec<Pubkey>;
    fn update(&mut self, accounts: &[KeyedAccountInterface]) -> Result<(), Self::Error>;
    fn get_all_specs(&self) -> AllSpecs<Self::Variant>;
    fn get_specs_for_operation(&self, op: &Self::Operation) -> AllSpecs<Self::Variant>;
}
```

## Program-Side Implementation (AmmSdk Example)

```rust
// In program crate: src/amm_test/sdk.rs (feature-gated)

pub enum AmmOperation {
    Swap,
    Deposit,
    Withdraw,
}

pub struct AmmSdk {
    // Extracted from PoolState
    pool_state_pubkey: Option<Pubkey>,
    amm_config: Option<Pubkey>,
    token_0_mint: Option<Pubkey>,
    token_1_mint: Option<Pubkey>,
    token_0_vault: Option<Pubkey>,
    token_1_vault: Option<Pubkey>,
    lp_mint: Option<Pubkey>,
    observation_key: Option<Pubkey>,

    // Derived PDAs
    authority: Option<Pubkey>,
    lp_mint_signer: Option<Pubkey>,

    // Specs cache
    program_owned_specs: HashMap<Pubkey, ProgramOwnedSpec<RentFreeAccountVariant>>,
    ata_specs: HashMap<Pubkey, AtaSpec>,
    mint_specs: HashMap<Pubkey, MintSpec>,
}
```

### Key Implementation Details

1. **`from_keyed_accounts`**: Parse root state, extract all pubkeys stored in it:
   ```rust
   fn from_keyed_accounts(accounts: &[KeyedAccountInterface]) -> Result<Self, Self::Error> {
       // Parse PoolState discriminator
       // Deserialize PoolState
       // Extract: amm_config, token_0_vault, token_1_vault, lp_mint, etc.
       // Derive: authority PDA, lp_mint_signer PDA
       // Build initial PoolState spec
   }
   ```

2. **`get_accounts_to_update`**: Return pubkeys based on operation:
   ```rust
   AmmOperation::Swap => [token_0_vault, token_1_vault]
   AmmOperation::Deposit => [token_0_vault, token_1_vault, observation, lp_mint]
   ```

3. **`update`**: Parse accounts by discriminator or known pubkey:
   ```rust
   // Check if pubkey matches known vaults -> parse as token
   // Check discriminator -> parse as PoolState/ObservationState
   // Build variant with seed values from SDK cache
   ```

4. **`get_specs_for_operation`**: Filter cached specs:
   ```rust
   AmmOperation::Swap => [pool_state, token_0_vault, token_1_vault]
   AmmOperation::Deposit => [pool_state, vaults, observation] + lp_mint spec
   ```

## Client Usage

### Simple Client Pattern

```rust
use csdk_anchor_full_derived_test::amm_test::{AmmSdk, AmmOperation};
use light_compressible_client::{
    AccountInterfaceExt, CompressibleProgram, KeyedAccountInterface,
    create_load_instructions_from_specs
};

// 1. Fetch pool state
let pool_interface = rpc
    .get_account_info_interface(&pool_pubkey, &program_id)
    .await?;

// 2. Create SDK from pool state
let keyed_pool = KeyedAccountInterface::from_pda_interface(pool_interface);
let mut sdk = AmmSdk::from_keyed_accounts(&[keyed_pool])?;

// 3. Get accounts to fetch (SDK returns typed descriptors)
let to_fetch = sdk.get_accounts_to_update_typed(&AmmOperation::Deposit);

// 4. Fetch all accounts - unified method handles type dispatch internally
let keyed_accounts = rpc.get_multiple_account_interfaces(&to_fetch).await?;

// 5. Update SDK with fetched accounts
sdk.update(&keyed_accounts)?;

// 6. Get specs for operation
let specs = sdk.get_specs_for_operation(&AmmOperation::Deposit);

// 7. Build decompression instructions (if any cold)
if specs.has_cold() {
    let ixs = create_load_instructions_from_specs(
        &specs,
        program_id,
        fee_payer,
        compression_config,
        rent_sponsor,
        &rpc,
    ).await?;
    
    // Execute decompression
    rpc.create_and_send_transaction(&ixs, &fee_payer, &[&payer]).await?;
}

// 8. Now execute the actual program instruction
```

## Footguns / Gotchas

### 1. Root Account First

Always parse the root account (e.g., PoolState) first via `from_keyed_accounts`. The SDK extracts pubkeys from it that are needed for subsequent account parsing.

```rust
// BAD: Updating vault before pool_state
sdk.update(&[vault_interface])?;  // Error: PoolStateNotParsed

// GOOD: Pool state first
let mut sdk = AmmSdk::from_keyed_accounts(&[pool_interface])?;
sdk.update(&[vault_interface])?;  // OK: can now match pubkey
```

### 2. MintSpec Requires Mint Data

For cold mints, `MintSpec` must contain both `compressed` account and parsed `mint_data`. The proof doesn't contain account data.

```rust
// When building MintSpec for cold mint:
MintSpec::cold(
    cmint,
    mint_signer,
    compressed_address,
    compressed_account,  // From indexer
    parsed_mint_data,    // Deserialized from compressed_account.data
)
```

### 3. Specs Are Cached

`update()` is additive - it adds/updates specs in the cache. Call `get_specs_for_operation()` after all relevant accounts are updated.

### 4. Variant Seed Values

The `RentFreeAccountVariant` stored in `ProgramOwnedSpec.variant` contains seed values extracted from the SDK cache (e.g., `pool_state`, `token_0_mint`). These are used by `create_load_instructions_from_specs` to build the correct instruction data.

### 5. Feature Flag Required

The SDK module is behind a feature flag to avoid adding client dependencies to the on-chain program:

```toml
# Cargo.toml
[features]
client-sdk = ["light-compressible-client"]
```

## File Locations

```
sdk-libs/compressible-client/src/
    compressible_program.rs   # Trait + types
    load_accounts.rs          # create_load_instructions_from_specs()

sdk-tests/csdk-anchor-full-derived-test/src/
    amm_test/
        sdk.rs                # AmmSdk implementation
        mod.rs                # Exports (feature-gated)
```

## State Transition Diagram

```
                              SDK INTERNAL STATE
    ============================================================

    [Empty]
        |
        | from_keyed_accounts([pool])
        v
    [PoolState Parsed]
        - pool_state_pubkey: Some
        - amm_config, token_0_mint, token_1_mint: Some
        - token_0_vault, token_1_vault, lp_mint: Some (from PoolState fields)
        - authority, lp_mint_signer: Derived
        - program_owned_specs: { pool_state -> PoolStateSpec }
        |
        | update([vault_0, vault_1, observation])
        v
    [All Accounts Parsed]
        - program_owned_specs: {
            pool_state -> PoolStateSpec,
            vault_0 -> TokenVaultSpec,
            vault_1 -> TokenVaultSpec,
            observation -> ObservationSpec,
          }
        |
        | get_specs_for_operation(Deposit)
        v
    [Specs Returned]
        AllSpecs {
            program_owned: [pool, vault_0, vault_1, observation],
            atas: [],
            mints: [lp_mint] (if populated),
        }
```

## Comparison with Old Approach

### Old: Manual RentFreeDecompressAccount construction

```rust
let accounts = vec![
    RentFreeDecompressAccount::from_seeds(
        AccountInterface::from(&pool_interface),
        PoolStateSeeds { amm_config, token_0_mint, token_1_mint },
    )?,
    RentFreeDecompressAccount::from_ctoken(
        AccountInterface::from(&vault_0_interface),
        TokenAccountVariant::Token0Vault { pool_state, token_0_mint },
    )?,
    // ... repeat for each account
];

for account in accounts {
    let ixs = create_load_accounts_instructions(&[account], ...)?;
    // ...
}
```

### New: SDK-based approach

```rust
let mut sdk = AmmSdk::from_keyed_accounts(&[pool_keyed])?;
sdk.update(&[vault_0_keyed, vault_1_keyed, observation_keyed])?;

let specs = sdk.get_specs_for_operation(&AmmOperation::Deposit);
let ixs = create_load_instructions_from_specs(&specs, ...)?;
```

**Benefits:**
- No manual seed construction - SDK extracts from parsed state
- Operation-aware - only loads accounts needed for specific operation
- Aggregator-friendly - can combine specs from multiple pools
- Type-safe - `RentFreeAccountVariant` with correct seed values
