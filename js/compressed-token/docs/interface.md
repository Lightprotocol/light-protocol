# c-Token Interface Reference

Concise reference for the v3 interface surface: reads (`getAtaInterface`), loads (`loadAta`, `createLoadAtaInstructions`), and transfers (`transferInterface`, `createTransferInterfaceInstructions`).

## 1. API Surface

| Method                                       | Path            | Purpose                                                    |
| -------------------------------------------- | --------------- | ---------------------------------------------------------- |
| `getAtaInterface`                            | v3, unified     | Aggregate balance from hot/cold/SPL/T22 sources            |
| `getOrCreateAtaInterface`                    | v3              | Create ATA if missing, return interface                    |
| `createLoadAtaInstructions`                  | v3              | Instruction batches for loading cold/wrap into ATA         |
| `loadAta`                                    | v3              | Action: execute load, return signature                     |
| `createTransferInterfaceInstructions`        | v3              | Instruction builder for transfers                          |
| `transferInterface`                          | v3              | Action: load + transfer (destination must exist)           |
| `createLightTokenTransferInstruction`        | v3/instructions | Raw c-token transfer ix (no load/wrap, no decimals)        |
| `createLightTokenTransferCheckedInstruction` | v3/instructions | Light-token transfer with decimals (used by transfer flow) |

Unified (`/unified`): `wrap=true` forced (not configurable; unified export omits wrap from options). Standard (`v3`): `wrap=false` default.

## 2. State Model (owner, mint)

| Source                        | Count  | Program                |
| ----------------------------- | ------ | ---------------------- |
| Light Token ATA (hot)         | 0 or 1 | LIGHT_TOKEN_PROGRAM_ID |
| Light Token compressed (cold) | 0..N   | LIGHT_TOKEN_PROGRAM_ID |
| SPL Token ATA (hot)           | 0 or 1 | TOKEN_PROGRAM_ID       |
| Token-2022 ATA (hot)          | 0 or 1 | TOKEN_2022_PROGRAM_ID  |

Constraints: mint owned by one of SPL/T22 (never both). All four source types can coexist for a given (owner, mint).

## 3. Modes: Unified vs Standard

|              | Unified (`wrap=true`)                                                         | Standard (`wrap=false`, default)     |
| ------------ | ----------------------------------------------------------------------------- | ------------------------------------ |
| Balance read | ctoken-hot + ctoken-cold + SPL + T22                                          | depends on `programId`               |
| Load         | Decompress cold + Wrap SPL/T22                                                | Decompress cold only                 |
| Target       | c-token ATA                                                                   | determined by `programId` / ATA type |
| Transfer ix  | `createLightTokenTransferCheckedInstruction` (Light) or SPL `transferChecked` | dispatched by `programId`            |

### Standard mode `getAtaInterface` behavior by `programId`

| `programId`              | Sources aggregated                            |
| ------------------------ | --------------------------------------------- |
| `undefined` (default)    | ctoken-hot + ALL ctoken-cold (no SPL/T22)     |
| `LIGHT_TOKEN_PROGRAM_ID` | ctoken-hot + ALL ctoken-cold                  |
| `TOKEN_PROGRAM_ID`       | SPL hot + compressed cold (tagged `spl-cold`) |
| `TOKEN_2022_PROGRAM_ID`  | T22 hot + compressed cold (tagged `t22-cold`) |

Note: compressed cold accounts always have `owner = LIGHT_TOKEN_PROGRAM_ID` regardless of the original mint's token program. The `spl-cold` / `t22-cold` tagging is a display convention for non-unified reads.

### Standard mode load behavior by ATA type

| ATA type    | Target                   | Pool    |
| ----------- | ------------------------ | ------- |
| `ctoken`    | c-token ATA (direct)     | No pool |
| `spl`       | SPL ATA (via token pool) | Yes     |
| `token2022` | T22 ATA (via token pool) | Yes     |

### Standard mode transfer dispatch

`createTransferInterfaceInstructions` dispatches the transfer instruction based on `programId`:

| `programId`              | Transfer instruction                         |
| ------------------------ | -------------------------------------------- |
| `LIGHT_TOKEN_PROGRAM_ID` | `createLightTokenTransferCheckedInstruction` |
| `TOKEN_PROGRAM_ID`       | `createTransferCheckedInstruction` (SPL)     |
| `TOKEN_2022_PROGRAM_ID`  | `createTransferCheckedInstruction` (T22)     |

For SPL/T22 with `wrap=false`: derives SPL/T22 ATAs, decompresses cold to SPL/T22 ATA via pool, then issues a standard SPL `transferChecked`. The flow is fully contained to SPL/T22 -- no Light token accounts involved.

## 4. Flow Diagrams

### getAtaInterface Dispatch

```
getAtaInterface(rpc, ata, owner, mint, commitment?, programId?, wrap?, allowOwnerOffCurve?)
    |
    +- programId=undefined (default)
    |   +- wrap=true -> getUnifiedAccountInterface
    |   |       -> ctoken-hot + ctoken-cold + SPL hot + T22 hot
    |   +- wrap=false -> getUnifiedAccountInterface
    |           -> ctoken-hot + ctoken-cold only (SPL/T22 NOT fetched)
    |
    +- programId=LIGHT_TOKEN -> getLightTokenAccountInterface
    |       -> ctoken-hot + ctoken-cold
    |
    +- programId=SPL|T22 -> getSplOrToken2022AccountInterface
            -> SPL/T22 hot (if exists) + compressed cold (as spl-cold/t22-cold)
```

### Load Path (\_buildLoadBatches)

```
_buildLoadBatches(rpc, payer, ata, options?, wrap, targetAta, targetAmount?, authority?, decimals)
    |
    +- checkNotFrozen(ata) -> throw if any source frozen (no selective skip)
    +- spl/t22/cold balance = 0 -> []
    |
    +- wrap=true
    |   +- Create c-token ATA (idempotent, if needed)
    |   +- Wrap SPL (if splBal>0)
    |   +- Wrap T22 (if t22Bal>0)
    |   +- Chunk cold by tree version (V2 only; assertV2Only rejects V1)
    |
    +- wrap=false
    |   +- Create target ATA (ctoken/SPL/T22 per ataType, idempotent)
    |   +- Chunk cold by tree version
    |
    +- For each chunk: fetch proof, build decompress ix
       assertUniqueInputHashes(chunks) <- hash uniqueness enforced
```

### Transfer Flow (createTransferInterfaceInstructions)

```
createTransferInterfaceInstructions(rpc, payer, mint, amount, sender, destination, decimals, options?)
    |
    +- amount <= 0 -> throw
    +- destination: token account address (must exist; derive via getAssociatedTokenAddressInterface)
    +- getAtaInterface(sender, wrap, programId)
    +- checkNotFrozen(senderInterface) -> throw if any source frozen
    +- balance < amount -> throw (Insufficient balance. Required: X, Available: Y)
    |
    +- _buildLoadBatches(..., decimals) -> internalBatches
    |
    +- programId = SPL|T22 && !wrap -> createTransferCheckedInstruction (SPL)
    +- else                         -> createLightTokenTransferCheckedInstruction (Light)
    |
    +- Returns TransactionInstruction[][]:
    +- batches.length = 0 (hot) -> [[CU, transferIx]]
    +- batches.length = 1       -> [[CU, ...batch0, transferIx]]
    +- batches.length > 1
        -> [[CU, load0], [CU, load1], ..., [CU, ...lastBatch, transferIx]]
        -> send [0..n-2] in parallel, then [n-1] after all confirm
```

### transferInterface (action)

```
transferInterface(rpc, payer, source, mint, destination, owner, amount, programId?, confirmOptions?, options?, wrap?, decimals?)
    |
    +- Validate source == getAssociatedTokenAddressInterface(mint, owner, programId)
    +- destination: token account address (must exist; derive via getAssociatedTokenAddressInterface)
    +- batches = createTransferInterfaceInstructions(..., destination, decimals, { ...options, wrap, programId })
    +- { rest: loads, last: transferIxs } = sliceLast(batches)
    +- Send loads in parallel (if any)
    +- Send transferIxs
```

## 5. Frozen Account Handling

SPL Token behavior: `getAccount()` returns full balance + `isFrozen=true`. The on-chain program rejects `transfer` for frozen accounts. There is no client-side pre-check in `@solana/spl-token`.

Light Token interface behavior:

| Method                                | Frozen accounts behavior                                                                                                       |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `getAtaInterface`                     | Shows full balance including frozen. `_anyFrozen=true`.                                                                        |
| `_buildLoadBatches`                   | Throws via `checkNotFrozen(ata)` at entry if any source is frozen; no selective skip.                                          |
| `createTransferInterfaceInstructions` | Throws via `checkNotFrozen(senderInterface)` if any source frozen. Insufficient balance error reports Required/Available only. |
| `transferInterface`                   | Same as above (delegates to `createTransferInterfaceInstructions`).                                                            |

Why throw instead of letting on-chain fail: a frozen account in a later batch would fail on-chain while earlier batches succeed, creating partial-load state. Early assert avoids this.

## 6. Delegate Handling

Compressed `TokenData` has `delegate: Option<Pubkey>` but no top-level `delegated_amount`; amount can come from TLV. `convertTokenDataToAccount` sets `delegatedAmount`: (1) from CompressedOnly extension TLV if present, (2) else if delegate set then full account amount, (3) else 0.

`buildAccountInterfaceFromSources`: `parsed.delegate` is the canonical delegate: if any hot source has a delegate, that delegate wins and delegated amount is summed across all sources for that delegate; else if only cold sources have delegates, the first cold delegate is canonical and amount is summed over cold. Multi-source canonicalDelegate logic, not "primary source only".

For load/transfer: `_buildLoadBatches` iterates `_sources` directly. Each cold account retains its own delegate info through the decompress instruction (`createDecompressInterfaceInstruction` includes delegate pubkeys in `packedAccountIndices`).

## 7. Hash Uniqueness Guarantee

Within a single call: compressed accounts fetched once globally, partitioned by tree version, each hash in exactly one batch. Enforced by `assertUniqueInputHashes`.

Across concurrent calls for the same sender: not serialized. Both calls read the same hashes from `rpc.getCompressedTokenAccountsByOwner`. First tx nullifies them on-chain, second tx fails with stale hashes. This is inherent to the UTXO/nullifier model (same as Bitcoin double-spend protection). Application-level serialization required for concurrent same-sender transfers.

## 8. Scenario Matrix (Unified, wrap=true)

| Sender           | Recipient  | Status                            |
| ---------------- | ---------- | --------------------------------- |
| Hot only         | ATA exists | Works                             |
| Hot only         | No ATA     | Fails (destination must exist)    |
| Cold <=8         | ATA exists | Works                             |
| Cold >8          | ATA exists | Works (parallel loads + transfer) |
| Cold             | No ATA     | Fails (destination must exist)    |
| Hot + Cold       | Any        | Works                             |
| SPL hot only     | Any        | Works (wrap)                      |
| SPL + Cold       | Any        | Works                             |
| Hot + SPL + Cold | Any        | Works                             |
| Nothing          | Any        | Throw: insufficient               |
| All frozen       | Any        | Throw: frozen                     |
| Partial frozen   | Any        | Throw: frozen (any source frozen) |
| amount=0         | Any        | Throw: zero amount                |
| Delegated cold   | Any        | Works                             |

### Standard (wrap=false) with programId

| programId | Sender state | Result                                              |
| --------- | ------------ | --------------------------------------------------- |
| Light     | cold only    | Decompress to c-token ATA + Light transfer          |
| Light     | hot only     | Light transfer directly                             |
| Light     | hot + cold   | Decompress + Light transfer                         |
| SPL       | cold only    | Create SPL ATA + decompress via pool + SPL transfer |
| SPL       | hot only     | SPL transferChecked directly                        |
| SPL       | hot + cold   | Decompress to SPL ATA + SPL transferChecked         |

## 9. Cases NOT Covered (Audit)

### Test coverage gaps

| Case                                               | Status                                 |
| -------------------------------------------------- | -------------------------------------- |
| Frozen sender (partial and full)                   | No e2e test                            |
| Zero-amount transfer rejection                     | No e2e test                            |
| Unified transfer (wrap=true) SPL hot-only sender   | No explicit e2e                        |
| Unified transfer SPL hot + cold                    | No explicit e2e                        |
| V1 tree in transfer path                           | No V1-specific test (V2 only in suite) |
| Self-transfer (sender == recipient)                | No test (allowed, consolidation)       |
| createTransferInterfaceInstructions with wrap=true | payment-flows uses wrap=false          |
| programId=SPL, cold-only transfer                  | Tested in transfer-interface.test.ts   |
| programId=SPL, hot-only transfer                   | Tested in transfer-interface.test.ts   |
| programId=SPL, instruction builder                 | Tested in transfer-interface.test.ts   |

### Design / out-of-scope

| Case                                               | Notes                                               |
| -------------------------------------------------- | --------------------------------------------------- |
| Two independent calls, same sender (e.g. two tabs) | Requires app-level locking; SDK has no shared state |
