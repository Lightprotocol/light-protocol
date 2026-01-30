# Init Flow: Visual Analysis & Gap Fill

## Side-by-Side Flow Comparison

### Current Test Code Flow

```
┌────────────────────────────────────────────────────────────────────────────┐
│ test_amm_full_lifecycle (current)                                          │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  ┌─────────────────────────────────────────────────────────────────┐       │
│  │ derive_amm_pdas()  // 50 lines of manual PDA derivation         │       │
│  │   - pool_state, pool_state_bump                                 │       │
│  │   - observation_state, observation_state_bump                   │       │
│  │   - authority, authority_bump                                   │       │
│  │   - token_0_vault, token_0_vault_bump                          │       │
│  │   - ... etc                                                     │       │
│  └───────────────────────────────┬─────────────────────────────────┘       │
│                                  │                                         │
│                                  ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────┐       │
│  │ get_create_accounts_proof()                                     │       │
│  │   vec![                                                         │       │
│  │     CreateAccountsProofInput::pda(pdas.pool_state),      // ❌  │       │
│  │     CreateAccountsProofInput::pda(pdas.observation_state),      │       │
│  │     CreateAccountsProofInput::mint(pdas.lp_mint_signer), // ❌  │       │
│  │   ]                                                             │       │
│  │                                                                 │       │
│  │   ❌ Client must know: which accounts need proofs               │       │
│  │   ❌ Client must know: pda() vs mint() distinction              │       │
│  │   ❌ Hidden: address tree fetch + address derivation            │       │
│  └───────────────────────────────┬─────────────────────────────────┘       │
│                                  │                                         │
│                                  ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────┐       │
│  │ Build instruction manually                                      │       │
│  │   - Repeat all pubkeys from pdas                                │       │
│  │   - Use bumps from pdas                                         │       │
│  │   - Attach remaining_accounts from proof                        │       │
│  └─────────────────────────────────────────────────────────────────┘       │
│                                                                            │
│  Problems:                                                                 │
│  1. PDA derivation duplicated (could be in SDK)                            │
│  2. Proof input selection requires protocol knowledge                      │
│  3. No type safety for instruction building                                │
│  4. Address derivation hidden inside get_create_accounts_proof             │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

### Ideal Design Flow

```
┌────────────────────────────────────────────────────────────────────────────┐
│ test_amm_full_lifecycle (ideal)                                            │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  ┌─────────────────────────────────────────────────────────────────┐       │
│  │ // Get address tree once (cacheable)                            │       │
│  │ let address_tree = rpc.get_address_tree_v2().tree;              │       │
│  │                                                                 │       │
│  │ ✅ Explicit dependency                                          │       │
│  │ ✅ Cacheable by aggregators                                     │       │
│  └───────────────────────────────┬─────────────────────────────────┘       │
│                                  │                                         │
│                                  ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────┐       │
│  │ AmmSdk::derive_init_pool(&config, &m0, &m1, &creator, &tree)    │       │
│  │                                                                 │       │
│  │ Returns InitPoolDerived {                                       │       │
│  │   accounts: InitPoolAccounts {                                  │       │
│  │     pool_state, pool_state_bump,         // ✅ All in one       │       │
│  │     observation_state, observation_state_bump,                  │       │
│  │     lp_mint_signer, lp_mint_signer_bump,                       │       │
│  │     lp_mint,                             // ✅ Derived too      │       │
│  │     token_0_vault, token_0_vault_bump,                         │       │
│  │     token_1_vault, token_1_vault_bump,                         │       │
│  │     creator_lp_token, creator_lp_token_bump,                   │       │
│  │     authority, authority_bump,                                  │       │
│  │   },                                                            │       │
│  │   proof_inputs: [                        // ✅ Pre-selected     │       │
│  │     { address, tree, name: "pool_state" },                      │       │
│  │     { address, tree, name: "observation_state" },               │       │
│  │     { address, tree: MINT_TREE, name: "lp_mint" },             │       │
│  │   ],                                                            │       │
│  │ }                                                               │       │
│  │                                                                 │       │
│  │ ✅ SYNC - no RPC                                                │       │
│  │ ✅ SDK knows which accounts need proofs                         │       │
│  │ ✅ Typed struct with bumps                                      │       │
│  │ ✅ Debug names included                                         │       │
│  └───────────────────────────────┬─────────────────────────────────┘       │
│                                  │                                         │
│                                  ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────┐       │
│  │ get_create_accounts_proof_from_addresses(                       │       │
│  │   &rpc, &program_id, derived.addresses_with_trees()             │       │
│  │ )                                                               │       │
│  │                                                                 │       │
│  │ ✅ Uses pre-computed addresses                                  │       │
│  │ ✅ No address derivation hidden                                 │       │
│  │ ✅ Batchable for aggregators                                    │       │
│  └───────────────────────────────┬─────────────────────────────────┘       │
│                                  │                                         │
│                                  ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────┐       │
│  │ // Type-safe instruction building                               │       │
│  │ let accounts = derived.accounts.to_anchor_accounts(...);        │       │
│  │ let params = derived.accounts.to_params(proof, 1000, 1000);     │       │
│  │                                                                 │       │
│  │ ✅ No manual pubkey wiring                                      │       │
│  │ ✅ Bumps auto-populated                                         │       │
│  │ ✅ Compile-time type checking                                   │       │
│  └─────────────────────────────────────────────────────────────────┘       │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## Gap Analysis: What Each Design Misses

### Current (v1 spec)

```
┌─────────────────────────────────────────────────────────────────┐
│ GAP: Client must know proof selection rules                     │
│                                                                 │
│   "Only PDAs and Mints need proofs, not token accounts"         │
│                                                                 │
│   ❌ Not documented clearly                                     │
│   ❌ Easy to get wrong                                          │
│   ❌ Protocol detail leaked to client                           │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│ GAP: PDA derivation duplicated                                  │
│                                                                 │
│   Client: derive_amm_pdas()      // 50 lines                    │
│   SDK:    (internal derivation)  // Another 50 lines            │
│                                                                 │
│   ❌ DRY violation                                              │
│   ❌ Risk of divergence                                         │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│ GAP: No type safety for instruction building                    │
│                                                                 │
│   InitializePool {                                              │
│     pool_state: pdas.pool_state,        // manual               │
│     observation_state: pdas.observation_state,                  │
│     ...                                 // error-prone          │
│   }                                                             │
│                                                                 │
│   ❌ Can miss accounts                                          │
│   ❌ Can use wrong pubkey                                       │
└─────────────────────────────────────────────────────────────────┘
```

### Design A (Manifest)

```
┌─────────────────────────────────────────────────────────────────┐
│ GAP: No bumps                                                   │
│                                                                 │
│   ManifestEntry { pubkey, role, name }                          │
│                                                                 │
│   ❌ Can't build instruction params                             │
│   ❌ Must re-derive for bumps                                   │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│ GAP: Generic manifest, not type-safe                            │
│                                                                 │
│   manifest.get("pool_state")  // returns Option<Pubkey>         │
│                                                                 │
│   ❌ String-based lookup                                        │
│   ❌ No compile-time checking                                   │
│   ❌ Typos fail at runtime                                      │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│ GAP: to_proof_inputs() hides address derivation                 │
│                                                                 │
│   let inputs = manifest.to_proof_inputs();                      │
│                                                                 │
│   ❌ Still some magic                                           │
│   ❌ Can't batch before derivation                              │
└─────────────────────────────────────────────────────────────────┘
```

### Design B (Raw Inputs)

```
┌─────────────────────────────────────────────────────────────────┐
│ GAP: No debug names                                             │
│                                                                 │
│   RawAddressInputs {                                            │
│     new_addresses: [AddressWithTree, ...]                       │
│   }                                                             │
│                                                                 │
│   ❌ Hard to debug which address failed                         │
│   ❌ No audit trail                                             │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│ GAP: High verbosity for simple cases                            │
│                                                                 │
│   let tree = rpc.get_address_tree_v2().tree;                    │
│   let derived = AmmSdk::derive_init_pool(..., &tree);           │
│   let proof = rpc.get_validity_proof(...).await?;               │
│   let packed = pack_proof(...)?;                                │
│   // ... more steps                                             │
│                                                                 │
│   ❌ 6+ steps for simple init                                   │
│   ❌ Overwhelming for beginners                                 │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│ GAP: No convenience layer                                       │
│                                                                 │
│   ❌ Every client must implement full flow                      │
│   ❌ No "just works" option                                     │
└─────────────────────────────────────────────────────────────────┘
```

---

## Ideal Design: How It Fills All Gaps

```
┌─────────────────────────────────────────────────────────────────┐
│ FILLED: SDK knows proof selection                               │
│                                                                 │
│   InitPoolDerived.proof_inputs                                  │
│   - Pre-populated by SDK                                        │
│   - Client never selects                                        │
│   ✅ Protocol knowledge encapsulated                            │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│ FILLED: Single source of truth for PDAs                         │
│                                                                 │
│   let derived = AmmSdk::derive_init_pool(...);                  │
│   - All PDAs in derived.accounts                                │
│   - All bumps included                                          │
│   - Client uses directly                                        │
│   ✅ No duplication                                             │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│ FILLED: Type-safe instruction building                          │
│                                                                 │
│   derived.accounts.to_anchor_accounts(...)                      │
│   - Compiler checks all fields                                  │
│   - Bumps auto-populated                                        │
│   ✅ Compile-time safety                                        │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│ FILLED: Debug names for addresses                               │
│                                                                 │
│   AddressProofInput { address, tree, name: "pool_state" }       │
│   - derived.log_proof_inputs()                                  │
│   ✅ Debuggable, auditable                                      │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│ FILLED: Convenience layer for simple cases                      │
│                                                                 │
│   let (derived, proof) = derive_and_prove_init_pool(&rpc, ...).await?;│
│   - One call for simple use                                     │
│   - Power users use raw derive                                  │
│   ✅ Both audiences served                                      │
└─────────────────────────────────────────────────────────────────┘
```

---

## Aggregator-Specific Flows

### Jupiter Integration

```
┌────────────────────────────────────────────────────────────────────────────┐
│ Jupiter: Multi-AMM Routing                                                 │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  // Jupiter aggregates multiple AMMs                                       │
│  let amms: Vec<Box<dyn Amm>> = fetch_amms(&route).await?;                  │
│                                                                            │
│  // For Light Protocol AMMs, use our SDK                                   │
│  let tree = jupiter_cache.address_tree();  // Cached                       │
│                                                                            │
│  for amm in amms {                                                         │
│      if let Some(light_amm) = amm.as_any().downcast_ref::<LightAmm>() {   │
│          // Use ideal design pattern                                       │
│          let derived = light_amm.derive_accounts(&tree);                   │
│                                                                            │
│          // Log for audit                                                  │
│          jupiter_logger.log_proof_inputs(&derived.proof_inputs);           │
│                                                                            │
│          // Batch addresses                                                │
│          all_addresses.extend(derived.addresses_with_trees());             │
│      }                                                                     │
│  }                                                                         │
│                                                                            │
│  // Batch proof fetch (Jupiter's own prover)                               │
│  let proofs = jupiter_prover.batch(all_addresses).await?;                  │
│                                                                            │
│  // Build instructions using typed accounts                                │
│  for (derived, proof) in deriveds.iter().zip(proofs) {                     │
│      let ix = build_ix(&derived.accounts, proof);                          │
│      route_ixs.push(ix);                                                   │
│  }                                                                         │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘

✅ Full control over proof batching
✅ Cacheable address tree
✅ Audit logging with names
✅ Type-safe instruction building
```

### DFlow Integration

```
┌────────────────────────────────────────────────────────────────────────────┐
│ DFlow: Order Flow with Custom Proof Infra                                  │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  // DFlow processes orders through their system                            │
│  async fn process_order(order: &Order) -> Result<Ix> {                     │
│      let tree = self.address_tree_cache.get();                             │
│                                                                            │
│      // Derive accounts (SYNC - fast)                                      │
│      let derived = AmmSdk::derive_init_pool(                               │
│          &order.config, &order.mint_0, &order.mint_1,                      │
│          &order.creator, &tree                                             │
│      );                                                                    │
│                                                                            │
│      // DFlow has their own proof infrastructure                           │
│      let addresses = derived.addresses_with_trees();                       │
│                                                                            │
│      // Custom proof generation (maybe GPU-accelerated)                    │
│      let proof = self.dflow_prover.generate(addresses).await?;             │
│                                                                            │
│      // Type-safe instruction                                              │
│      Ok(derived.accounts.to_instruction(proof))                            │
│  }                                                                         │
│                                                                            │
│  // Batch processing                                                       │
│  async fn process_batch(orders: &[Order]) -> Result<Vec<Ix>> {             │
│      // Derive all (SYNC - parallelizable)                                 │
│      let deriveds: Vec<_> = orders                                         │
│          .par_iter()                                                       │
│          .map(|o| AmmSdk::derive_init_pool(...))                           │
│          .collect();                                                       │
│                                                                            │
│      // Batch all addresses                                                │
│      let all_addresses: Vec<_> = deriveds                                  │
│          .iter()                                                           │
│          .flat_map(|d| d.addresses_with_trees())                           │
│          .collect();                                                       │
│                                                                            │
│      // Single proof batch (efficient)                                     │
│      let proofs = self.prover.batch(all_addresses).await?;                 │
│                                                                            │
│      // Build all instructions                                             │
│      Ok(deriveds.iter().zip(proofs).map(|(d, p)| d.to_ix(p)).collect())    │
│  }                                                                         │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘

✅ SYNC derivation for parallelization
✅ Custom proof infrastructure
✅ Batch-friendly address extraction
✅ No forced RPC patterns
```

---

## Summary: Design Decisions

| Decision | Rationale |
|----------|-----------|
| Typed PDA struct | Type safety > flexibility |
| Bumps included | Required for instruction params |
| address_tree param | Explicit > hidden RPC |
| AddressProofInput with name | Debugging is non-negotiable |
| addresses_with_trees() | Raw protocol types for batching |
| to_anchor_accounts() | Type-safe instruction building |
| Optional convenience wrapper | Support both simple and advanced users |
| SYNC derive, ASYNC proof | Clear separation of concerns |
