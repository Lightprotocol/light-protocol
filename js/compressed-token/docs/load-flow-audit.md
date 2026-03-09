# Load flow audit: why it’s complex and how to align with Solana UX

## Normal Solana UX (reference)

- **One account:** User has an ATA (owner + mint → one address). `getAccountInfo` or SPL `getAccount()` → balance.
- **One action:** Transfer = one instruction, one tx. No “load” step.
- **One program:** SPL Token (or Token-2022). No “which program?” or “wrap/unwrap”.

So: **one address → one balance → one tx per action.**

---

## Why the load flow is not that

### 1. Dual state (hot vs cold)

Tokens can live in two places:

- **Hot:** On-chain light-token account (like a normal ATA). One `getAccountInfo` → balance.
- **Cold:** Compressed (in Merkle tree). No single account; many “leaves” per owner+mint. Need indexer/RPC (`getCompressedTokenAccountsByOwner`) to list them.

So “your balance” is not one fetch. It’s **hot balance + sum(cold leaves)**. The SDK hides this behind `getAtaInterface` (aggregate). That’s the first layer of complexity: unified read over multiple sources.

### 2. Load-before-use

To **spend** cold balance, it must become hot first (decompress). So:

- **Normal Solana:** “Transfer 10” = one transfer tx.
- **Compressed:** “Transfer 10” can mean: (1) load 10 from cold into hot (one or more decompress txs), (2) then transfer 10 (one tx). So “one user action” can be **N + 1 txs**.

That’s the second layer: **load is a separate, multi-tx step** before the “real” action.

### 3. Batch limit (8 inputs per decompress)

Program limit: one decompress instruction can take at most 8 compressed inputs. So:

- 20 cold leaves → 3 chunks → 3 load txs (or 3 batches in one flow).
- Need chunking, proof per chunk, ordering (parallel vs sequential). That’s the third layer: **batching and proof fetching**.

### 4. Three token worlds (SPL, Token-2022, light-token)

We have:

- SPL ATA, Token-2022 ATA, light-token ATA (three addresses for same owner+mint).
- “Unified” mode: treat SPL/T22 as extra sources and **wrap** them into light-token so “one balance” includes SPL + T22 + light (hot + cold).

So we get:

- **programId** and **wrap** and **target ATA type** (light vs SPL vs T22).
- Different transfer instructions (light-token transfer vs SPL `transferChecked`).
- Decompress can target light-token ATA **or** SPL/T22 ATA (via pool). More branches in `_buildLoadBatches`.

That’s the fourth layer: **multi-program and wrap/unwrap**.

### 5. Options and call-site variation

To support all of the above we added:

- `wrap` (include SPL/T22 in balance and in load).
- `programId` (which token program / which ATA we care about).
- `targetAta` (where we’re loading to).
- `targetAmount` (for transfer: only load what’s needed).
- `interfaceOptions` (owner override, pre-fetched SPL interface info).
- `ensureRecipientAta`, `sequentialLoad`, etc.

So the same “transfer” or “load” has many code paths. That’s the fifth layer: **combinatorics of options**.

---

## Where the complexity lives (map)

| Layer | What | Why it exists |
|-------|------|----------------|
| **Unified read** | `getAtaInterface`, sources (hot/cold/SPL/T22), priority, `_sources` | One “balance” from many places. |
| **Load engine** | `_buildLoadBatches`, chunking, proofs, setup vs decompress | Turn cold (and optionally SPL/T22) into hot (or into SPL/T22 ATA). |
| **Instruction builders** | `createLoadAtaInstructions`, `createTransferInterfaceInstructions`, `createUnwrapInstructions` | Assemble 0..N load batches + final action. |
| **Action wrappers** | `loadAta`, `transferInterface`, `unwrap` | Sign + send N txs (parallel or sequential). |
| **Unified vs standard** | unified index (`wrap=true` default) vs v3 actions (`wrap=false` default) | Two APIs for “one balance” vs “one program only”. |
| **ATA abstraction** | `getAssociatedTokenAddressInterface`, `createAssociatedTokenAccountInterfaceIdempotentInstruction`, `checkAtaAddress` | One API for three program types. |

The “mess” is the **stacking** of these: dual state → load step → batching → multi-program + wrap → many options → two entry points (unified vs v3).

---

## How to keep it closer to Solana UX (simplify)

### Principle

**Default path = one mental model:** “I have a balance (we aggregate under the hood). I do an action. You send the right txs.” Minimize visible concepts (load, wrap, programId, batches) for the common case.

### 1. One default “token” for apps: light-token only

- **Default:** No SPL/T22 in the balance. No `wrap`. No `programId` in the API.  
  So: one ATA type (light-token), one balance = hot + cold (we aggregate), one transfer instruction when we’re done loading.
- **Optional:** Explicit “unified” or “wrap” mode for apps that need SPL/T22 in the same flow. Keep it behind a single opt-in (e.g. `unified: true` or `wrap: true`), not the default.

Effect: Most apps see “one program, one balance, load+transfer as one operation.” Complexity stays behind one flag.

### 2. Hide “load” in the action; expose “instructions” for power users

- **Default:** `transferInterface(...)` means “transfer this amount”; internally we do load (if needed) + transfer. Caller doesn’t see “load” or batch count unless they inspect return or logs.
- **Power users:** Keep `createTransferInterfaceInstructions` (and optionally a “plan” or batch count) for wallets that need to show “2 txs” or estimate fees. So the complexity is **available but not required**.

Effect: Solana-like API: “transfer(amount)”. Multi-tx is an implementation detail unless you ask for instructions/plan.

### 3. Single entry point for “balance + actions”

- Prefer **one** main entry (e.g. unified or a single “light token” facade) that:
  - `getBalance(owner, mint)` → aggregated (hot + cold).
  - `transfer(payer, owner, mint, recipient, amount)` → does load (if needed) + transfer.
  - `unwrap(...)` → load (if needed) + unwrap.
- Deprecate or narrow the surface where callers must choose between “v3” and “unified” and pass `wrap`/`programId` everywhere.

Effect: Fewer “modes” and fewer knobs for the common case.

### 4. Reduce options in the default path

- **In defaults:** No `interfaceOptions` unless delegate/owner override is needed. No `splInterfaceInfos` (fetch when needed). No `targetAmount` in the public API for transfer (we compute it from amount).
- **Keep options** for: delegate, custom payer, confirm options, and one “advanced” object for the rest (wrap, programId, sequential load, etc.).

Effect: Simple call sites; complexity behind one “options” bag and only when needed.

### 5. Sequential load by default; parallel as opt-in

- Send load batches **sequentially** by default (same ATA = same account, avoid partial load on conflict).
- Optional `parallelLoad: true` for advanced use. Document that parallel is best-effort and may require retries.

Effect: Predictable behavior; fewer “it sometimes failed” reports.

### 6. Document the “Solana-like” contract

- In docs: “Default behavior: one balance (hot + cold), one action (transfer/unwrap). We may send more than one tx under the hood. Order and count are deterministic from (owner, mint, amount).”
- List the few cases where we deviate (e.g. multi-batch load, unwrap to SPL). So auditors and power users know what to expect.

---

## Summary

| Why it’s complex | Root cause | Simplification |
|------------------|------------|----------------|
| Many sources for “balance” | Hot + cold + optional SPL/T22 | Default = light-token only; aggregate hot+cold only. |
| Load before transfer | Compressed must decompress first | Keep load internal to transfer/unwrap; don’t expose as separate concept by default. |
| Many txs for one action | Chunking (8 inputs) + proofs | Accept N+1 txs internally; expose “instructions” or “plan” only when needed. |
| programId / wrap / targetAta | Three programs, unified mode | One default (light-token); wrap/unified behind one opt-in. |
| Options explosion | Flexibility for all combos | Minimal defaults; one “advanced” options object. |
| Parallel load | Latency vs correctness | Sequential by default; parallel opt-in and documented. |

---

## Async calls in load / transfer / unwrap flows

| Where | Call | Purpose |
|-------|------|--------|
| **get-account-interface** | `rpc.getAccountInfo(address)` | Fetch hot SPL/T22/light-token account. |
| **get-account-interface** | `rpc.getCompressedTokenAccountsByOwner(...)` | List cold accounts for owner+mint. |
| **get-account-interface** | `Promise.allSettled` / `Promise.all` | Parallel hot + cold fetches. |
| **load-ata** | `_getAtaInterface(...)` | Aggregate balance; builds source list. |
| **load-ata** | `_buildLoadBatches(...)` | Build load batches. |
| **_buildLoadBatches** | `getSplInterfaceInfos(rpc, mint)` | SPL interface PDA + token program (wrap / decompress-to-SPL). |
| **_buildLoadBatches** | `getMint(rpc, mint, ..., tokenProgram)` | **Decimals only** when `options.decimals` not set. |
| **_buildLoadBatches** | `rpc.getValidityProofV0(proofInputs)` per chunk | ZK validity proofs per decompress chunk. |
| **transfer-interface** | `_getAtaInterface(...)` | Sender balance/sources. |
| **transfer-interface** | `_buildLoadBatches(...)` | Load batches. |
| **transfer-interface** | `getMint` / `getMintInterface` | **Decimals only** when `options.decimals` not set. |
| **unwrap** | `getSplInterfaceInfos(rpc, mint)` | Resolve SPL interface if not passed in. |
| **unwrap** | `rpc.getAccountInfo(destination)` | Ensure destination ATA exists. |
| **unwrap** | `_getAtaInterface(...)` | Source balance/sources. |
| **unwrap** | `_buildLoadBatches(...)` | Load batches. |
| **unwrap** | `getMint(rpc, mint, ..., tokenProgram)` | **Decimals only** when `interfaceOptions.decimals` not set. |

Passing `decimals` (e.g. `InterfaceOptions.decimals`) removes all four decimals-only RPC calls in those flows.

**Target:** “As close to Solana UX as possible” = one balance, one action (transfer/unwrap), one or a few txs under the hood, with complexity opt-in and documented.
