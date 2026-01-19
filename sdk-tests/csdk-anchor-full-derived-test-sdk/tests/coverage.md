# LightProgramInterface Trait Test Coverage Plan

## Overview

Comprehensive test coverage for the `LightProgramInterface` trait to ensure robust SDK implementations.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    TEST COVERAGE ARCHITECTURE                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐              │
│  │  UNIT TESTS     │  │  INTEGRATION    │  │  PROPERTY       │              │
│  │  (Trait Methods)│  │  (Multi-Op)     │  │  (Invariants)   │              │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘              │
│           │                    │                    │                       │
│           v                    v                    v                       │
│  ┌────────────────────────────────────────────────────────────┐             │
│  │               LightProgramInterface Trait                    │             │
│  │  ┌──────────────────┐  ┌──────────────────┐               │             │
│  │  │from_keyed_accounts│  │get_accounts_to_  │               │             │
│  │  │                  │  │update            │               │             │
│  │  └──────────────────┘  └──────────────────┘               │             │
│  │  ┌──────────────────┐  ┌──────────────────┐               │             │
│  │  │update            │  │get_all_specs     │               │             │
│  │  │                  │  │                  │               │             │
│  │  └──────────────────┘  └──────────────────┘               │             │
│  │  ┌──────────────────┐                                     │             │
│  │  │get_specs_for_    │                                     │             │
│  │  │operation         │                                     │             │
│  │  └──────────────────┘                                     │             │
│  └────────────────────────────────────────────────────────────┘             │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 1. Core Trait Method Tests

### 1.1 `from_keyed_accounts()` Tests

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     from_keyed_accounts() Test Matrix                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  INPUT                      │  EXPECTED                  │  TEST NAME       │
│  ─────────────────────────────────────────────────────────────────────────  │
│  Empty accounts []          │  Err or empty SDK          │  empty_accounts  │
│  Single root (PoolState)    │  SDK with extracted pubkeys│  single_root     │
│  Multiple roots             │  SDK with merged state     │  multiple_roots  │
│  Wrong discriminator        │  Skip or error             │  wrong_disc      │
│  Truncated data             │  ParseError                │  truncated_data  │
│  Hot root account           │  SDK (no cold_context)     │  hot_root        │
│  Cold root account          │  SDK with cold_context     │  cold_root       │
│  Missing required fields    │  ParseError                │  missing_fields  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

| Test ID | Test Name | Description | Priority |
|---------|-----------|-------------|----------|
| T1.1.1 | `test_from_keyed_empty_accounts` | Empty array returns error/empty SDK | HIGH |
| T1.1.2 | `test_from_keyed_single_root` | Single PoolState parses all pubkeys | HIGH |
| T1.1.3 | `test_from_keyed_cold_root` | Cold root sets up cold_context correctly | HIGH |
| T1.1.4 | `test_from_keyed_hot_root` | Hot root works without cold_context | HIGH |
| T1.1.5 | `test_from_keyed_wrong_discriminator` | Unknown discriminator handled gracefully | MEDIUM |
| T1.1.6 | `test_from_keyed_truncated_data` | Insufficient data returns ParseError | HIGH |
| T1.1.7 | `test_from_keyed_zero_length_data` | Zero-length data handled | MEDIUM |
| T1.1.8 | `test_from_keyed_multiple_roots` | Multiple root accounts merged correctly | MEDIUM |

### 1.2 `get_accounts_to_update()` Tests

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                Operation -> Accounts Mapping Test Matrix                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   OPERATION      │  EXPECTED ACCOUNTS          │  OVERLAP WITH OTHERS       │
│  ──────────────────────────────────────────────────────────────────────────│
│   Swap           │  [vault_0, vault_1]         │  Subset of Deposit         │
│   Deposit        │  [vault_0, vault_1, obs,    │  Superset of Swap          │
│                  │   lp_mint]                  │                            │
│   Withdraw       │  [vault_0, vault_1, obs,    │  Same as Deposit           │
│                  │   lp_mint]                  │                            │
│                                                                             │
│   EDGE CASES:                                                               │
│   - Before pool_state parsed → returns []                                   │
│   - Pool has no vaults → returns [] for Swap                                │
│   - Pool has no LP mint → Deposit excludes it                               │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

| Test ID | Test Name | Description | Priority |
|---------|-----------|-------------|----------|
| T1.2.1 | `test_get_accounts_swap` | Swap returns correct vaults | HIGH |
| T1.2.2 | `test_get_accounts_deposit` | Deposit returns vaults+obs+mint | HIGH |
| T1.2.3 | `test_get_accounts_withdraw` | Withdraw matches Deposit | HIGH |
| T1.2.4 | `test_get_accounts_before_init` | Returns empty before pool parsed | HIGH |
| T1.2.5 | `test_get_accounts_overlap` | Verify overlapping accounts deduplicated | MEDIUM |
| T1.2.6 | `test_get_accounts_partial_state` | Missing some optional fields | MEDIUM |

### 1.3 `update()` Tests

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          update() State Transitions                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   INITIAL STATE           INPUT               FINAL STATE                   │
│  ──────────────────────────────────────────────────────────────────────────│
│   [PoolState parsed]  +   [vault_0]      →    specs: {pool, vault_0}       │
│   specs: {pool}                                                             │
│                                                                             │
│   [PoolState parsed]  +   [vault_0,      →    specs: {pool, vault_0,       │
│   specs: {pool}           vault_1]            vault_1}                      │
│                                                                             │
│   specs: {pool, v0}   +   [vault_0]      →    specs: {pool, v0} (updated)  │
│   (already has v0)        (re-update)         IDEMPOTENT                    │
│                                                                             │
│   specs: {}           +   [vault_0]      →    ERROR (pool not parsed)      │
│   (no pool yet)                                                             │
│                                                                             │
│   specs: {pool}       +   [unknown]      →    specs: {pool} (skipped)      │
│                           (unrecognized)                                    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

| Test ID | Test Name | Description | Priority |
|---------|-----------|-------------|----------|
| T1.3.1 | `test_update_single_account` | Single vault updates correctly | HIGH |
| T1.3.2 | `test_update_multiple_accounts` | Multiple accounts batch | HIGH |
| T1.3.3 | `test_update_idempotent` | Same account twice is idempotent | HIGH |
| T1.3.4 | `test_update_before_root` | Error if updating before root parsed | HIGH |
| T1.3.5 | `test_update_unknown_account` | Unknown accounts skipped | MEDIUM |
| T1.3.6 | `test_update_mixed_hot_cold` | Mix of hot and cold accounts | HIGH |
| T1.3.7 | `test_update_overwrites_old` | Re-updating changes is_cold status | HIGH |
| T1.3.8 | `test_update_token_context` | Token accounts use token_context | HIGH |
| T1.3.9 | `test_update_pda_context` | PDA accounts use pda_context | HIGH |

### 1.4 `get_all_specs()` Tests

| Test ID | Test Name | Description | Priority |
|---------|-----------|-------------|----------|
| T1.4.1 | `test_get_all_empty` | Empty SDK returns empty specs | HIGH |
| T1.4.2 | `test_get_all_complete` | All parsed accounts returned | HIGH |
| T1.4.3 | `test_get_all_preserves_cold` | Cold status preserved in specs | HIGH |
| T1.4.4 | `test_get_all_categories` | Correct categorization (pda/ata/mint) | HIGH |

### 1.5 `get_specs_for_operation()` Tests

```
┌─────────────────────────────────────────────────────────────────────────────┐
│               Operation-Filtered Specs Visual                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ALL SPECS:                                                                │
│   ┌──────────────────────────────────────────────────────────┐              │
│   │ pool_state │ vault_0 │ vault_1 │ observation │ lp_mint │              │
│   └──────────────────────────────────────────────────────────┘              │
│                                                                             │
│   SWAP FILTER:                                                              │
│   ┌──────────────────────────────────────────────────────────┐              │
│   │ pool_state │ vault_0 │ vault_1 │░░░░░░░░░░░░│░░░░░░░░░│              │
│   └──────────────────────────────────────────────────────────┘              │
│                    ↑ INCLUDED       ↑ EXCLUDED                              │
│                                                                             │
│   DEPOSIT FILTER:                                                           │
│   ┌──────────────────────────────────────────────────────────┐              │
│   │ pool_state │ vault_0 │ vault_1 │ observation │ lp_mint │              │
│   └──────────────────────────────────────────────────────────┘              │
│                    ↑ ALL INCLUDED                                           │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

| Test ID | Test Name | Description | Priority |
|---------|-----------|-------------|----------|
| T1.5.1 | `test_specs_for_swap` | Swap returns vaults only | HIGH |
| T1.5.2 | `test_specs_for_deposit` | Deposit includes all | HIGH |
| T1.5.3 | `test_specs_for_operation_cold_filter` | Only cold accounts have context | HIGH |
| T1.5.4 | `test_specs_for_operation_missing_accounts` | Missing accounts not in specs | MEDIUM |

---

## 2. Error Handling Tests

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        ERROR SCENARIOS MATRIX                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ERROR TYPE              │  SCENARIO                │  EXPECTED MESSAGE    │
│  ──────────────────────────────────────────────────────────────────────────│
│   ParseError              │  Invalid account data    │  "Parse error: ..."  │
│   UnknownDiscriminator    │  Unrecognized disc       │  "Unknown disc: [..]"│
│   MissingField            │  Required field null     │  "Missing: field_x"  │
│   PoolStateNotParsed      │  Update before init      │  "Pool state must..."│
│   MissingContext          │  Cold without context    │  "Missing context"   │
│                                                                             │
│   RECOVERY SCENARIOS:                                                       │
│   - Partial parse failure → previously parsed state preserved               │
│   - Unknown account → skip silently, continue                               │
│   - Hot account missing context → OK (no context needed)                    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

| Test ID | Test Name | Description | Priority |
|---------|-----------|-------------|----------|
| T2.1 | `test_error_parse_invalid_data` | ParseError on invalid data | HIGH |
| T2.2 | `test_error_missing_field` | MissingField with field name | HIGH |
| T2.3 | `test_error_pool_not_parsed` | PoolStateNotParsed meaningful msg | HIGH |
| T2.4 | `test_error_display_impl` | All errors have Display impl | HIGH |
| T2.5 | `test_error_recovery_partial` | Partial failure preserves state | MEDIUM |
| T2.6 | `test_error_cold_without_context` | Cold account without context errors | HIGH |

---

## 3. Multi-Operation Scenarios (Overlapping/Divergent Accounts)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│             MULTI-OPERATION ACCOUNT OVERLAP SCENARIOS                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   SCENARIO: Sequential Operations with Shared Accounts                      │
│                                                                             │
│   Timeline:                                                                 │
│   ─────────────────────────────────────────────────────────────────────────│
│   T0: Initialize SDK with PoolState                                         │
│       └── specs: {pool_state}                                               │
│                                                                             │
│   T1: get_accounts_to_update(Swap) → [vault_0, vault_1]                     │
│       └── Fetch and update vaults                                           │
│       └── specs: {pool_state, vault_0, vault_1}                             │
│                                                                             │
│   T2: get_specs_for_operation(Swap) → {pool, v0, v1}                        │
│       └── Execute Swap with these specs                                     │
│                                                                             │
│   T3: get_accounts_to_update(Deposit) → [vault_0, vault_1, obs, lp_mint]    │
│       └── Already have vaults! Only need obs + lp_mint                      │
│       └── Fetch obs + lp_mint, update                                       │
│       └── specs: {pool_state, vault_0, vault_1, obs, lp_mint}               │
│                                                                             │
│   T4: get_specs_for_operation(Deposit) → {pool, v0, v1, obs, lp_mint}       │
│                                                                             │
│   KEY INVARIANT: Shared accounts (vaults) use SAME spec instance            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

| Test ID | Test Name | Description | Priority |
|---------|-----------|-------------|----------|
| T3.1 | `test_multi_op_swap_then_deposit` | Specs preserved across ops | HIGH |
| T3.2 | `test_multi_op_shared_accounts` | Shared accounts not duplicated | HIGH |
| T3.3 | `test_multi_op_incremental_fetch` | Can skip already-fetched accounts | HIGH |
| T3.4 | `test_multi_op_state_refresh` | Re-fetching updates cold→hot | HIGH |
| T3.5 | `test_multi_op_interleaved` | Alternating ops work correctly | MEDIUM |

---

## 4. Account Naming / Aliasing Tests

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    ACCOUNT NAMING EDGE CASES                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   PROBLEM: Same account address, different instruction names                 │
│                                                                             │
│   Example:                                                                   │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │  Instruction: initialize                                            │   │
│   │    accounts:                                                        │   │
│   │      - token_vault_0: CYLaS4pMLTb1gTrxf9YnMNkF6ta7vMopKgST5kDAWdU2 │   │
│   │      - pool_state: 8qitTUf7KWgEwgsLnSfrt52GfTAcUmFRci4h5RdnJh5m    │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │  Instruction: swap                                                  │   │
│   │    accounts:                                                        │   │
│   │      - source_vault: CYLaS4pMLTb1gTrxf9YnMNkF6ta7vMopKgST5kDAWdU2  │  <── SAME!
│   │      - amm_pool: 8qitTUf7KWgEwgsLnSfrt52GfTAcUmFRci4h5RdnJh5m      │  <── SAME!
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│   SOLUTION: SDK keyed by PUBKEY, not name                                    │
│   - HashMap<Pubkey, Spec> ensures same address = same spec                  │
│   - Variant enum contains canonical data, not instruction-specific names    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

| Test ID | Test Name | Description | Priority |
|---------|-----------|-------------|----------|
| T4.1 | `test_same_address_different_name` | Same pubkey = same spec | HIGH |
| T4.2 | `test_spec_keyed_by_pubkey` | HashMap uses pubkey not name | HIGH |
| T4.3 | `test_variant_canonical_data` | Variant has canonical seeds | HIGH |
| T4.4 | `test_instruction_agnostic` | Works regardless of ix context | MEDIUM |

---

## 5. Exhaustive Coverage Requirements

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                   EXHAUSTIVE IMPLEMENTATION REQUIREMENTS                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   A valid LightProgramInterface implementation MUST:                          │
│                                                                             │
│   1. VARIANT COMPLETENESS                                                   │
│      □ LightAccountVariant covers ALL #[light_account] accounts             │
│      □ TokenAccountVariant covers ALL #[rentfree_token] accounts            │
│      □ No rentfree account left unrepresented                               │
│                                                                             │
│   2. OPERATION COMPLETENESS                                                 │
│      □ Operation enum covers all instruction types                          │
│      □ Each operation returns correct account set                           │
│      □ get_specs_for_operation returns superset of get_accounts_to_update   │
│                                                                             │
│   3. SEED VALUE COMPLETENESS                                                │
│      □ All seed fields populated from parsed state                          │
│      □ Variant constructor includes all seed values                         │
│      □ Seeds match what macros expect for address derivation                │
│                                                                             │
│   4. CONTEXT COMPLETENESS                                                   │
│      □ Cold accounts have appropriate context (Pda/Token/Mint)              │
│      □ Hot accounts have no context (or empty)                              │
│      □ Context types match account types                                    │
│                                                                             │
│   VALIDATION CHECKS TO IMPLEMENT:                                           │
│                                                                             │
│   ┌───────────────────────────────────────────────────────────────────┐     │
│   │  fn validate_implementation<T: LightProgramInterface>() {           │     │
│   │      // 1. Create SDK from known root                             │     │
│   │      // 2. For each Operation:                                    │     │
│   │      //    - get_accounts_to_update returns non-empty             │     │
│   │      //    - After update, get_specs_for_operation non-empty      │     │
│   │      //    - All specs have valid variants                        │     │
│   │      // 3. get_all_specs covers all accounts from all ops         │     │
│   │  }                                                                │     │
│   └───────────────────────────────────────────────────────────────────┘     │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

| Test ID | Test Name | Description | Priority |
|---------|-----------|-------------|----------|
| T5.1 | `test_variant_covers_all_rentfree` | No rentfree account missing from variant | HIGH |
| T5.2 | `test_operation_covers_all_instructions` | All ix types have operation | HIGH |
| T5.3 | `test_seeds_complete` | All seed values populated | HIGH |
| T5.4 | `test_context_type_matches` | PDA→PdaContext, Token→TokenContext | HIGH |
| T5.5 | `test_all_specs_superset` | get_all_specs ⊇ union of all get_specs_for_op | HIGH |
| T5.6 | `test_no_orphan_accounts` | Every program account reachable via some op | MEDIUM |

---

## 6. Property-Based / Invariant Tests

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         SDK INVARIANTS                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   INVARIANT 1: Idempotency                                                  │
│   ─────────────────────────────────────────────────────────────────────────│
│   ∀ accounts a: update(a); update(a) ≡ update(a)                            │
│   (updating with same data twice has same effect as once)                   │
│                                                                             │
│   INVARIANT 2: Commutativity                                                │
│   ─────────────────────────────────────────────────────────────────────────│
│   update([a, b]) ≡ update([a]); update([b]) ≡ update([b]); update([a])     │
│   (order of updates doesn't matter for final state)                         │
│                                                                             │
│   INVARIANT 3: Spec Consistency                                             │
│   ─────────────────────────────────────────────────────────────────────────│
│   ∀ op: get_accounts_to_update(op) ⊆ keys(get_specs_for_operation(op))     │
│   (all accounts to update should appear in specs after update)              │
│                                                                             │
│   INVARIANT 4: Address Uniqueness                                           │
│   ─────────────────────────────────────────────────────────────────────────│
│   ∀ specs: |specs.addresses| = |unique(specs.addresses)|                    │
│   (no duplicate addresses in specs)                                         │
│                                                                             │
│   INVARIANT 5: Cold Context Presence                                        │
│   ─────────────────────────────────────────────────────────────────────────│
│   ∀ spec: spec.is_cold ⟹ spec.cold_context.is_some()                       │
│   (cold specs must have context)                                            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

| Test ID | Test Name | Description | Priority |
|---------|-----------|-------------|----------|
| T6.1 | `test_invariant_idempotent` | update(a);update(a) = update(a) | HIGH |
| T6.2 | `test_invariant_commutative` | Order doesn't matter | HIGH |
| T6.3 | `test_invariant_spec_consistency` | Accounts in specs after update | HIGH |
| T6.4 | `test_invariant_no_duplicates` | No duplicate addresses | HIGH |
| T6.5 | `test_invariant_cold_has_context` | Cold specs have context | HIGH |
| T6.6 | `test_invariant_hot_no_context_needed` | Hot specs work without context | MEDIUM |

---

## 7. State Transition Tests

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      STATE TRANSITION DIAGRAM                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                        ┌──────────────┐                                     │
│                        │   EMPTY      │                                     │
│                        │   SDK        │                                     │
│                        └──────┬───────┘                                     │
│                               │                                             │
│                               │ from_keyed_accounts([pool])                 │
│                               │ (parses root)                               │
│                               v                                             │
│                        ┌──────────────┐                                     │
│                        │  ROOT PARSED │                                     │
│                        │  (pool only) │                                     │
│                        └──────┬───────┘                                     │
│                               │                                             │
│            ┌──────────────────┼──────────────────┐                          │
│            │                  │                  │                          │
│            │ update([vaults]) │ update([obs])    │ update([mint])           │
│            v                  v                  v                          │
│     ┌──────────────┐   ┌──────────────┐   ┌──────────────┐                  │
│     │ SWAP READY   │   │ PARTIAL      │   │ MINT READY   │                  │
│     │ (vaults)     │   │ (vaults+obs) │   │ (mint)       │                  │
│     └──────────────┘   └──────────────┘   └──────────────┘                  │
│            │                  │                  │                          │
│            └──────────────────┼──────────────────┘                          │
│                               │ update([remaining])                         │
│                               v                                             │
│                        ┌──────────────┐                                     │
│                        │   COMPLETE   │                                     │
│                        │  (all specs) │                                     │
│                        └──────────────┘                                     │
│                                                                             │
│   TRANSITIONS:                                                              │
│   - Any state → COMPLETE (by updating remaining accounts)                   │
│   - Hot → Cold (account compressed externally, re-fetch)                    │
│   - Cold → Hot (account decompressed, re-fetch)                             │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

| Test ID | Test Name | Description | Priority |
|---------|-----------|-------------|----------|
| T7.1 | `test_state_empty_to_root` | Empty → Root parsed | HIGH |
| T7.2 | `test_state_root_to_swap_ready` | Root → Swap ready (vaults) | HIGH |
| T7.3 | `test_state_incremental_to_complete` | Incremental updates to complete | HIGH |
| T7.4 | `test_state_hot_to_cold_refetch` | Re-fetch changes hot→cold | HIGH |
| T7.5 | `test_state_cold_to_hot_refetch` | Re-fetch changes cold→hot | HIGH |

---

## 8. Edge Case Tests

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         EDGE CASES MATRIX                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   SCENARIO                     │ EXPECTED BEHAVIOR        │ TEST            │
│  ──────────────────────────────────────────────────────────────────────────│
│   Pool with zero vaults        │ Swap returns empty       │ zero_vaults     │
│   Pool without LP mint         │ Deposit excludes mint    │ no_lp_mint      │
│   All accounts hot             │ all_hot() = true         │ all_hot         │
│   All accounts cold            │ has_cold() = true        │ all_cold        │
│   Mixed hot/cold               │ correct filtering        │ mixed_state     │
│   Very large state data        │ Handles without OOM      │ large_data      │
│   Concurrent updates           │ No race conditions       │ concurrent      │
│   Null pubkeys in state        │ Graceful handling        │ null_pubkeys    │
│   Duplicate accounts in update │ Deduplicated             │ duplicate_accts │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

| Test ID | Test Name | Description | Priority |
|---------|-----------|-------------|----------|
| T8.1 | `test_edge_zero_vaults` | Pool with no vaults | MEDIUM |
| T8.2 | `test_edge_no_lp_mint` | Pool without LP mint | MEDIUM |
| T8.3 | `test_edge_all_hot` | all_hot() works correctly | HIGH |
| T8.4 | `test_edge_all_cold` | has_cold() works correctly | HIGH |
| T8.5 | `test_edge_mixed_hot_cold` | Mixed state handled | HIGH |
| T8.6 | `test_edge_duplicate_accounts` | Duplicates deduplicated | MEDIUM |

---

## 9. Same Type Different Instance Tests (CRITICAL)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│         SAME TYPE, DIFFERENT INSTANCE - SPEC SEPARATION                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   SCENARIO: vault_0 and vault_1 are BOTH TokenVault type                    │
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │  VAULT_0                         VAULT_1                           │   │
│   │  ─────────────────────────────────────────────────────────────────  │   │
│   │  pubkey: 0xAAAA...               pubkey: 0xBBBB...                  │   │
│   │  type: Token0Vault               type: Token1Vault                  │   │
│   │  seeds: [pool, mint_0]           seeds: [pool, mint_1]              │   │
│   │                                                                     │   │
│   │          ↓ DIFFERENT PUBKEYS = DIFFERENT SPECS ↓                   │   │
│   │                                                                     │   │
│   │  ┌───────────────────────────────────────────────────────────┐     │   │
│   │  │  HashMap<Pubkey, Spec>                                    │     │   │
│   │  │  ──────────────────────────────────────────────────────── │     │   │
│   │  │  0xAAAA... → Spec { variant: Token0Vault, ... }           │     │   │
│   │  │  0xBBBB... → Spec { variant: Token1Vault, ... }           │     │   │
│   │  └───────────────────────────────────────────────────────────┘     │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│   KEY INVARIANTS:                                                           │
│   1. Pubkey is globally unique → HashMap key guarantees no mingling         │
│   2. Variant enum encodes WHICH account via type + seed values              │
│   3. Field name (vault_0, vault_1) unique across ALL instructions           │
│   4. Updating vault_0 does NOT affect vault_1                               │
│   5. get_specs_for_operation returns ALL required instances                 │
│                                                                             │
│   CROSS-INSTRUCTION NAMING:                                                 │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │  initialize.token_0_vault ──────┐                                   │   │
│   │                                  ├──→ SAME pubkey = SAME spec       │   │
│   │  swap.input_vault ──────────────┘                                   │   │
│   │                                                                     │   │
│   │  SDK keys by PUBKEY, not field name, so same account                │   │
│   │  referenced by different names = single spec entry                  │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

| Test ID | Test Name | Description | Priority |
|---------|-----------|-------------|----------|
| T9.1 | `test_same_type_different_pubkey_separate_specs` | Two vaults with different pubkeys = two specs | CRITICAL |
| T9.2 | `test_variant_seed_values_distinguish_instances` | Variants contain different seed values | CRITICAL |
| T9.3 | `test_specs_contain_all_vaults_not_merged` | Specs returns BOTH vaults, not merged | CRITICAL |
| T9.4 | `test_field_name_uniqueness_across_instructions` | Same pubkey from different names = single spec | CRITICAL |
| T9.5 | `test_updating_vault_0_does_not_affect_vault_1` | Update isolation between vaults | CRITICAL |
| T9.6 | `test_operation_returns_all_required_instances` | Operation returns ALL needed instances | CRITICAL |
| T9.7 | `test_hashmap_keying_prevents_spec_mingling` | HashMap<Pubkey, Spec> prevents mingling | CRITICAL |

---

## Test Implementation Summary

### Total Tests by Category

| Category | Count | Priority HIGH | Priority CRITICAL |
|----------|-------|---------------|-------------------|
| 1. Core Methods | 22 | 18 | 0 |
| 2. Error Handling | 6 | 5 | 0 |
| 3. Multi-Operation | 5 | 4 | 0 |
| 4. Account Naming | 4 | 3 | 0 |
| 5. Exhaustive Coverage | 6 | 5 | 0 |
| 6. Invariants | 6 | 5 | 0 |
| 7. State Transitions | 5 | 5 | 0 |
| 8. Edge Cases | 6 | 3 | 0 |
| 9. Same Type Different Instance | 7 | 0 | **7** |
| **TOTAL** | **67** | **48** | **7** |

### Currently Implemented Tests: **31 PASSING**

```
test test_all_specs_helpers ... ok
test test_edge_all_hot_check ... ok
test test_error_missing_field_names_field ... ok
test test_error_display_impl ... ok
test test_edge_duplicate_accounts_in_update ... ok
test test_error_parse_error_contains_cause ... ok
test test_field_name_uniqueness_across_instructions ... ok           [T9.4]
test test_from_keyed_empty_accounts ... ok
test test_from_keyed_truncated_data ... ok
test test_from_keyed_wrong_discriminator ... ok
test test_from_keyed_zero_length_data ... ok
test test_get_accounts_before_init ... ok
test test_get_accounts_swap_vs_deposit ... ok
test test_get_accounts_to_update_typed_categories ... ok
test test_get_accounts_to_update_typed_empty ... ok
test test_get_all_empty ... ok
test test_hashmap_keying_prevents_spec_mingling ... ok               [T9.7]
test test_invariant_cold_has_context ... ok
test test_invariant_hot_context_optional ... ok
test test_invariant_no_duplicate_addresses ... ok
test test_multi_op_deposit_superset_of_swap ... ok
test test_multi_op_withdraw_equals_deposit ... ok
test test_operation_returns_all_required_instances ... ok            [T9.6]
test test_same_pubkey_same_spec ... ok
test test_same_type_different_pubkey_separate_specs ... ok           [T9.1]
test test_specs_contain_all_vaults_not_merged ... ok                 [T9.3]
test test_update_idempotent ... ok
test test_update_before_root_errors ... ok
test test_update_unknown_account_skipped ... ok
test test_updating_vault_0_does_not_affect_vault_1 ... ok            [T9.5]
test test_variant_seed_values_distinguish_instances ... ok           [T9.2]
```

### Implementation Priority Order

1. **Phase 0 (CRITICAL)**: T9.* (Same Type Different Instance - ALL IMPLEMENTED)
2. **Phase 1 (HIGH)**: T1.1.*, T1.3.*, T2.*, T6.* (Core + Error + Invariants)
3. **Phase 2 (IMPORTANT)**: T1.2.*, T1.4.*, T1.5.*, T3.*, T5.* (Ops + Multi-op)
4. **Phase 3 (ROBUSTNESS)**: T4.*, T7.*, T8.* (Naming + State + Edge)

---

## File Structure

```
sdk-tests/csdk-anchor-full-derived-test-sdk/
├── src/
│   └── lib.rs              # AmmSdk implementation
└── tests/
    └── trait_tests.rs      # All trait unit tests (31 tests)
```
