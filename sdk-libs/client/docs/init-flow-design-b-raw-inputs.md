# Init Flow Design B: Raw Inputs Pattern

## Philosophy

**Close to the metal**: Expose the exact data structures the protocol uses.
**Zero abstraction**: Client sees what the on-chain program sees.
**Aggregator-friendly**: Data flows linearly, easy to trace.

---

## Core Principle

Instead of abstracting proof inputs, expose the **raw protocol types** with helper derivation:

```rust
/// Raw addresses for proof generation.
/// This is what get_validity_proof actually needs.
#[derive(Debug, Clone)]
pub struct RawAddressInputs {
    /// Addresses that need non-inclusion proofs (new accounts)
    pub new_addresses: Vec<AddressWithTree>,
}

/// Derived PDAs with their bumps.
/// Client needs bumps for instruction data.
#[derive(Debug, Clone)]
pub struct DerivedPdas<T> {
    /// Program-specific PDA struct with all addresses + bumps
    pub pdas: T,
    /// Which addresses need proofs (indices into pdas)
    pub proof_addresses: RawAddressInputs,
}
```

---

## SDK Contract

Each SDK defines its own strongly-typed PDA struct:

```rust
/// All PDAs for InitializePool with bumps.
#[derive(Debug, Clone)]
pub struct InitPoolPdas {
    pub pool_state: Pubkey,
    pub pool_state_bump: u8,
    pub observation_state: Pubkey,
    pub observation_state_bump: u8,
    pub authority: Pubkey,
    pub authority_bump: u8,
    pub lp_mint_signer: Pubkey,
    pub lp_mint_signer_bump: u8,
    pub lp_mint: Pubkey,
    pub token_0_vault: Pubkey,
    pub token_0_vault_bump: u8,
    pub token_1_vault: Pubkey,
    pub token_1_vault_bump: u8,
    pub creator_lp_token: Pubkey,
    pub creator_lp_token_bump: u8,
}

impl AmmSdk {
    /// Derive all PDAs and identify which need proofs.
    /// 
    /// SYNC - no RPC. Returns raw protocol inputs.
    /// 
    /// The `address_tree` is required because address derivation
    /// depends on the tree. Get it from `rpc.get_address_tree_v2()`.
    pub fn derive_init_pool(
        amm_config: &Pubkey,
        token_0_mint: &Pubkey,
        token_1_mint: &Pubkey,
        creator: &Pubkey,
        address_tree: &Pubkey,  // Client provides from RPC
    ) -> DerivedPdas<InitPoolPdas> {
        // Derive all PDAs...
        let (pool_state, pool_state_bump) = derive_pool_state(...);
        // ... other derivations ...
        
        let pdas = InitPoolPdas {
            pool_state,
            pool_state_bump,
            // ... all fields ...
        };
        
        // Derive compressed addresses for accounts that need proofs
        let pool_address = derive_address(&pool_state.to_bytes(), &address_tree.to_bytes(), &PROGRAM_ID.to_bytes());
        let obs_address = derive_address(&observation_state.to_bytes(), &address_tree.to_bytes(), &PROGRAM_ID.to_bytes());
        let mint_address = derive_mint_compressed_address(&lp_mint_signer, &MINT_ADDRESS_TREE);
        
        DerivedPdas {
            pdas,
            proof_addresses: RawAddressInputs {
                new_addresses: vec![
                    AddressWithTree { address: pool_address, tree: *address_tree },
                    AddressWithTree { address: obs_address, tree: *address_tree },
                    AddressWithTree { address: mint_address, tree: MINT_ADDRESS_TREE },
                ],
            },
        }
    }
}
```

---

## Client Flow

```rust
// 1. Get address tree (single RPC call)
let address_tree = rpc.get_address_tree_v2().tree;

// 2. Derive everything (SYNC)
let derived = AmmSdk::derive_init_pool(&config, &mint_0, &mint_1, &creator, &address_tree);

// 3. Get validity proof using raw addresses (ASYNC)
let validity_proof = rpc
    .get_validity_proof(vec![], derived.proof_addresses.new_addresses.clone(), None)
    .await?
    .value;

// 4. Pack proof (SYNC)
let state_tree_info = rpc.get_random_state_tree_info()?;
let packed = pack_proof(&program_id, validity_proof.clone(), &state_tree_info, None)?;

// 5. Build instruction with raw pdas and proof
let ix = build_init_pool_ix(
    &derived.pdas,           // Has all pubkeys + bumps
    validity_proof.proof,
    packed.address_trees[0], // Address tree info
    packed.output_tree_index,
);
```

---

## Why Expose address_tree?

The client already needs RPC for:
1. Getting validity proofs
2. Getting state tree info

Making `address_tree` explicit:
- Shows the dependency clearly
- Allows caching (address trees rarely change)
- No hidden RPC in SDK

```rust
// Client can cache address tree
let address_tree = match cached_address_tree {
    Some(tree) => tree,
    None => {
        let tree = rpc.get_address_tree_v2().tree;
        cache.set_address_tree(tree);
        tree
    }
};
```

---

## Aggregator Usage (Jupiter/DFlow)

```rust
// Jupiter wants raw control
let address_tree = rpc.get_address_tree_v2().tree;
let derived = AmmSdk::derive_init_pool(&config, &mint_0, &mint_1, &creator, &address_tree);

// They can inspect raw addresses
println!("Pool compressed address: {:?}", derived.proof_addresses.new_addresses[0].address);
println!("Pool state PDA: {}", derived.pdas.pool_state);

// They fetch proof their way (maybe batching with other proofs)
let validity_proof = their_proof_service.get_proof(
    derived.proof_addresses.new_addresses.clone()
).await?;

// Build instruction with their preferred method
let ix = their_ix_builder.build_init_pool(&derived.pdas, &validity_proof);
```

---

## Type Safety: Instruction Builder

SDK can provide type-safe instruction building:

```rust
impl InitPoolPdas {
    /// Build InitializePool accounts struct.
    pub fn to_accounts(&self, creator: &Pubkey, config: &Pubkey, mints: (&Pubkey, &Pubkey)) -> InitializePoolAccounts {
        InitializePoolAccounts {
            creator: *creator,
            amm_config: *config,
            authority: self.authority,
            pool_state: self.pool_state,
            token_0_mint: *mints.0,
            token_1_mint: *mints.1,
            lp_mint_signer: self.lp_mint_signer,
            lp_mint: self.lp_mint,
            token_0_vault: self.token_0_vault,
            token_1_vault: self.token_1_vault,
            observation_state: self.observation_state,
            creator_lp_token: self.creator_lp_token,
            // ... static accounts ...
        }
    }
    
    /// Build InitializeParams with proof data.
    pub fn to_params(&self, proof: CreateAccountsProof, init_amount_0: u64, init_amount_1: u64) -> InitializeParams {
        InitializeParams {
            init_amount_0,
            init_amount_1,
            open_time: 0,
            create_accounts_proof: proof,
            lp_mint_signer_bump: self.lp_mint_signer_bump,
            creator_lp_token_bump: self.creator_lp_token_bump,
            authority_bump: self.authority_bump,
        }
    }
}
```

---

## Trade-offs

### Pros
- **Zero abstraction**: Client sees exactly what protocol uses
- **Full control**: Client can batch, cache, or customize anything
- **Type-safe**: Program-specific PDA struct prevents errors
- **Predictable**: No hidden RPC, no magic derivation
- **Composable**: Raw types work with any proof fetching strategy

### Cons
- More verbose client code (but explicit)
- Client must call `get_address_tree_v2()` explicitly
- Multiple steps vs one-liner (but each step is transparent)

---

## Comparison with Design A

| Aspect | Design A (Manifest) | Design B (Raw Inputs) |
|--------|---------------------|----------------------|
| Abstraction | Light (roles + names) | None |
| Type safety | Generic manifest | Program-specific structs |
| address_tree | Hidden | Explicit parameter |
| Bump access | Optional | Built-in |
| Proof building | Helper method | Raw protocol types |
| Client verbosity | Medium | Higher |
| Customization | Medium | Maximum |

---

## Open Questions

1. Should `DerivedPdas<T>` include the address_tree used?
2. Should we provide a convenience wrapper for the 3-step proof fetch?
3. How to handle programs with variable number of init accounts?
