# Init Flow Design: Executive Summary

## The Problem

Clients initializing Light Protocol accounts must currently:
1. Manually derive all PDAs (50+ lines)
2. Know protocol rules: "PDAs and Mints need proofs, token accounts don't"
3. Correctly select `pda()` vs `mint()` for each account
4. Build instructions by manually wiring pubkeys

**Result**: Error-prone, protocol knowledge leaked to clients.

---

## Designs Evaluated

| Design | Core Idea | Aggregator Fit | Simple Client Fit |
|--------|-----------|----------------|-------------------|
| **v1 Spec** | Trait method returns proof inputs | Medium | High |
| **A: Manifest** | Flat list with roles + names | High | High |
| **B: Raw Inputs** | Protocol-native types, explicit tree | Maximum | Medium |
| **Ideal** | Typed structs + raw access | Maximum | High |

---

## Recommendation: Ideal Design

Combine typed structs (compile-time safety) with raw address access (aggregator control).

### API Surface

```rust
// SDK provides per-instruction typed struct
let accounts = InitPoolAccounts::derive(&config, &m0, &m1, &creator, &address_tree);

// Access proof inputs (pre-selected by SDK)
accounts.addresses_with_trees()  // -> Vec<AddressWithTree>
accounts.log_proof_inputs()      // -> debug output with names

// Type-safe instruction building
accounts.to_anchor_accounts(...)
accounts.to_params(proof, ...)
```

### Client Flow

```
┌──────────────────────────────┐
│ 1. rpc.get_address_tree_v2() │  ASYNC (cache this)
└──────────────┬───────────────┘
               │
               ▼
┌──────────────────────────────┐
│ 2. Accounts::derive(...)     │  SYNC
└──────────────┬───────────────┘
               │
               ▼
┌──────────────────────────────┐
│ 3. get_proof_for_addresses() │  ASYNC
└──────────────┬───────────────┘
               │
               ▼
┌──────────────────────────────┐
│ 4. Build instruction         │  SYNC (type-safe)
└──────────────────────────────┘
```

---

## Key Design Decisions

| Decision | Why |
|----------|-----|
| `address_tree` as explicit param | No hidden RPC, cacheable |
| Typed struct (not generic manifest) | Compile-time safety, IDE support |
| Bumps included | Required for instruction params |
| AddressProofInput with names | Debugging, audit trail |
| `addresses_with_trees()` method | Raw protocol types for batching |
| `to_anchor_accounts()` helper | Type-safe instruction building |

---

## Aggregator Benefits

**Jupiter**:
- Batch addresses across multiple AMMs
- Custom proof infrastructure
- Audit logging with names

**DFlow**:
- SYNC derivation for parallel processing
- Raw addresses for custom provers
- Full control over RPC patterns

---

## Migration Path

1. Add `AddressProofInput` type to `light_client::interface`
2. Add `get_proof_for_addresses()` function
3. SDK teams implement typed account structs per instruction
4. Update tests to use new pattern
5. Document for aggregators

---

## Documents

| File | Purpose |
|------|---------|
| `init-flow-spec.md` | Original v1 spec (reference) |
| `init-flow-design-a-manifest.md` | Design A details |
| `init-flow-design-b-raw-inputs.md` | Design B details |
| `init-flow-comparison.md` | Side-by-side comparison |
| `init-flow-visual-analysis.md` | Flow diagrams + gap analysis |
| `init-flow-ideal.md` | Combined ideal design |
| `init-flow-final-design.md` | Battle-tested implementation |
| `init-flow-executive-summary.md` | This document |
