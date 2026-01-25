# Init Flow Design Comparison

## Visual: Data Flow Diagrams

### Current Design (v1 spec)

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLIENT CODE                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   ┌──────────────────────────────────────────────────────┐      │
│   │ 1. sdk.get_create_accounts_inputs(&instruction)      │      │
│   │    (returns Vec<CreateAccountsProofInput>)           │      │
│   └────────────────────────┬─────────────────────────────┘      │
│                            │                                     │
│                            ▼                                     │
│   ┌──────────────────────────────────────────────────────┐      │
│   │ 2. get_create_accounts_proof(&rpc, &program_id, inputs) │   │
│   │    (ASYNC - does address tree fetch + proof fetch)   │      │
│   └────────────────────────┬─────────────────────────────┘      │
│                            │                                     │
│                            ▼                                     │
│   ┌──────────────────────────────────────────────────────┐      │
│   │ 3. Build instruction with proof_result               │      │
│   │    (client must still derive PDAs separately!)       │      │
│   └──────────────────────────────────────────────────────┘      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘

Problems:
- PDAs derived twice (in SDK method + client instruction building)
- Client doesn't see intermediate types
- "Magic" inside get_create_accounts_proof
```

### Design A: Account Manifest

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLIENT CODE                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   ┌──────────────────────────────────────────────────────┐      │
│   │ 1. AmmSdk::init_pool_manifest(config, m0, m1, creator)│      │
│   │    SYNC - returns AccountManifest                    │      │
│   │    ┌────────────────────────────────────────────┐    │      │
│   │    │ entries: [                                 │    │      │
│   │    │   { pool_state,     AddressedPda,  "..." } │    │      │
│   │    │   { observation,    AddressedPda,  "..." } │    │      │
│   │    │   { lp_mint_signer, AddressedMint, "..." } │    │      │
│   │    │   { token_0_vault,  TokenAccount,  "..." } │    │      │
│   │    │   { creator_lp,     Ata,           "..." } │    │      │
│   │    │ ]                                          │    │      │
│   │    └────────────────────────────────────────────┘    │      │
│   └────────────────────────┬─────────────────────────────┘      │
│                            │                                     │
│                            ▼                                     │
│   ┌──────────────────────────────────────────────────────┐      │
│   │ 2. manifest.to_proof_inputs()                        │      │
│   │    SYNC - filters AddressedPda | AddressedMint       │      │
│   └────────────────────────┬─────────────────────────────┘      │
│                            │                                     │
│                            ▼                                     │
│   ┌──────────────────────────────────────────────────────┐      │
│   │ 3. get_create_accounts_proof(&rpc, &pid, inputs)     │      │
│   │    ASYNC                                             │      │
│   └────────────────────────┬─────────────────────────────┘      │
│                            │                                     │
│                            ▼                                     │
│   ┌──────────────────────────────────────────────────────┐      │
│   │ 4. Build instruction using manifest.get("pool_state")│      │
│   │    SYNC - all pubkeys from manifest                  │      │
│   └──────────────────────────────────────────────────────┘      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘

Advantages:
- All accounts visible with roles
- Single source of truth for pubkeys
- Filtering is explicit client decision
```

### Design B: Raw Inputs

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLIENT CODE                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   ┌──────────────────────────────────────────────────────┐      │
│   │ 0. address_tree = rpc.get_address_tree_v2().tree     │      │
│   │    ASYNC - explicit dependency                        │      │
│   └────────────────────────┬─────────────────────────────┘      │
│                            │                                     │
│                            ▼                                     │
│   ┌──────────────────────────────────────────────────────┐      │
│   │ 1. AmmSdk::derive_init_pool(..., &address_tree)      │      │
│   │    SYNC - returns DerivedPdas<InitPoolPdas>          │      │
│   │    ┌────────────────────────────────────────────┐    │      │
│   │    │ pdas: InitPoolPdas {                       │    │      │
│   │    │   pool_state, pool_state_bump,             │    │      │
│   │    │   observation_state, observation_state_bump│    │      │
│   │    │   ...all pubkeys + bumps...                │    │      │
│   │    │ }                                          │    │      │
│   │    │ proof_addresses: RawAddressInputs {        │    │      │
│   │    │   new_addresses: [AddressWithTree, ...]    │    │      │
│   │    │ }                                          │    │      │
│   │    └────────────────────────────────────────────┘    │      │
│   └────────────────────────┬─────────────────────────────┘      │
│                            │                                     │
│                            ▼                                     │
│   ┌──────────────────────────────────────────────────────┐      │
│   │ 2. rpc.get_validity_proof([], new_addresses, None)   │      │
│   │    ASYNC - using raw protocol types                  │      │
│   └────────────────────────┬─────────────────────────────┘      │
│                            │                                     │
│                            ▼                                     │
│   ┌──────────────────────────────────────────────────────┐      │
│   │ 3. pack_proof(&pid, validity_proof, &state_tree_info)│      │
│   │    SYNC                                              │      │
│   └────────────────────────┬─────────────────────────────┘      │
│                            │                                     │
│                            ▼                                     │
│   ┌──────────────────────────────────────────────────────┐      │
│   │ 4. derived.pdas.to_accounts(...).to_account_metas()  │      │
│   │    SYNC - type-safe instruction building             │      │
│   └──────────────────────────────────────────────────────┘      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘

Advantages:
- Zero abstraction over protocol types
- Full control over every RPC call
- Type-safe PDA struct with bumps
```

---

## Comparison Matrix

| Criterion | v1 Spec | Design A | Design B |
|-----------|---------|----------|----------|
| **Transparency** | Low | High | Maximum |
| **Account visibility** | Partial | Full (with roles) | Full (with bumps) |
| **Hidden RPC** | Yes (in get_create_accounts_proof) | Partially | None |
| **Type safety** | Medium | Medium | High (typed PDA struct) |
| **Aggregator fit** | Medium | High | Maximum |
| **Client verbosity** | Low | Medium | Higher |
| **Bump access** | No | Optional | Built-in |
| **Debugging** | Hard | Easy (names) | Easy (types) |
| **Customization** | Low | Medium | Maximum |
| **Learning curve** | Low | Low | Medium |

---

## Jupiter AMM Trait Comparison

Jupiter's `Amm` trait:

```rust
trait Amm {
    fn from_keyed_account(keyed_account: &KeyedAccount, amm_context: &AmmContext) -> Result<Self>;
    fn get_accounts_to_update(&self) -> Vec<Pubkey>;
    fn update(&mut self, account_map: &AccountMap) -> Result<()>;
    fn quote(&self, quote_params: &QuoteParams) -> Result<Quote>;
    fn get_swap_and_account_metas(&self, swap_params: &SwapParams) -> Result<SwapAndAccountMetas>;
}
```

Key patterns:
1. **Flat return types**: `Vec<Pubkey>`, not nested abstractions
2. **Sync/Async split**: `get_accounts_to_update()` is sync, fetching is client's job
3. **Explicit update**: Client feeds data via `update()`
4. **Single responsibility**: Each method does one thing

**Design A** aligns with Jupiter's `get_accounts_to_update()` pattern (flat list with metadata).

**Design B** goes further, exposing raw protocol types for maximum control.

---

## Aggregator Requirements Analysis

### Jupiter Integration

```rust
// Jupiter wants:
// 1. Know all accounts upfront (for simulation)
// 2. Batch fetches across multiple AMMs
// 3. Audit trail / logging

// Design A fits well:
let manifest = AmmSdk::init_pool_manifest(&config, &m0, &m1, &creator);
jupiter_logger.log_accounts(&manifest.entries);

// Design B fits well too:
let derived = AmmSdk::derive_init_pool(&config, &m0, &m1, &creator, &tree);
jupiter_logger.log_pdas(&derived.pdas);
```

### DFlow Integration

```rust
// DFlow wants:
// 1. Deterministic address derivation
// 2. Proof batching across orders
// 3. Custom proof infrastructure

// Design B is ideal:
let derived = AmmSdk::derive_init_pool(&config, &m0, &m1, &creator, &tree);

// Batch proofs across multiple init operations
let all_addresses: Vec<AddressWithTree> = orders
    .iter()
    .flat_map(|o| o.derived.proof_addresses.new_addresses.clone())
    .collect();

let batched_proof = dflow_prover.batch_proof(all_addresses).await?;
```

---

## Recommendation

**For aggregators**: Design B (Raw Inputs) provides maximum control.

**For typical clients**: Design A (Manifest) balances transparency with ease of use.

**Hybrid approach**: Implement Design B as the foundation, provide Design A as a convenience layer:

```rust
impl AccountManifest {
    /// Create manifest from raw derived PDAs.
    pub fn from_derived<T: IntoManifestEntries>(derived: &DerivedPdas<T>) -> Self {
        AccountManifest {
            entries: derived.pdas.into_manifest_entries(),
        }
    }
}
```

---

## Next Steps

1. Validate designs against actual aggregator requirements
2. Prototype both designs with the AMM test
3. Get feedback from Jupiter/DFlow teams
4. Choose or hybridize based on real-world usage
