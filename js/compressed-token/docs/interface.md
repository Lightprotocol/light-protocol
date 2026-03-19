# c-Token Interface Reference

Concise v3 interface reference for reads (`getAtaInterface`), loads
(`loadAta`, `createLoadAtaInstructions`), and transfers.

## 1. API Surface

| Method                                         | Path        | Purpose                                                          |
| ---------------------------------------------- | ----------- | ---------------------------------------------------------------- |
| `getAtaInterface`                              | v3, unified | Aggregate balance from hot/cold/SPL/T22 sources                 |
| `getOrCreateAtaInterface`                      | v3          | Create ATA if missing, return interface                         |
| `createLoadAtaInstructions`                    | v3          | Build load/decompress batches                                   |
| `loadAta`                                      | v3          | Execute load/decompress batches                                 |
| `transferInterface`                            | v3          | Action: transfer to recipient wallet (ATA derived/ensured)      |
| `createTransferInterfaceInstructions`          | v3          | Builder: recipient wallet input, destination ATA derived/ensured |
| `transferToAccountInterface`                   | v3          | Action: explicit destination token account (legacy behavior)    |
| `createTransferToAccountInterfaceInstructions` | v3          | Builder: explicit destination token account                     |
| `createLightTokenTransferInstruction`          | instructions| Raw light-token transfer ix (no load/wrap)                     |

Unified (`/unified`) always forces `wrap=true`.
Standard (`v3`) defaults to `wrap=false`.

## 2. Transfer API Split

### Wallet-recipient path (default payments UX)

```ts
transferInterface(
  rpc,
  payer,
  source,
  mint,
  recipientWallet,
  owner,
  amount,
  programId?,
  confirmOptions?,
  options?,
  wrap?,
  decimals?,
)
```

```ts
createTransferInterfaceInstructions(
  rpc,
  payer,
  mint,
  amount,
  sender,
  recipientWallet,
  decimals,
  options?,
)
```

- Derives destination ATA from recipient wallet.
- Inserts idempotent ATA create into final transfer batch.
- Supports SPL/T22/light-token dispatch via `programId`.

### Explicit destination-account path

```ts
transferToAccountInterface(
  rpc,
  payer,
  source,
  mint,
  destinationTokenAccount,
  owner,
  amount,
  programId?,
  confirmOptions?,
  options?,
  wrap?,
  decimals?,
)
```

```ts
createTransferToAccountInterfaceInstructions(
  rpc,
  payer,
  mint,
  amount,
  sender,
  destinationTokenAccount,
  decimals,
  options?,
)
```

- Preserves previous destination-account semantics.
- Use for custom/program-owned destination accounts.

## 3. Off-Curve/PDA Recipient Note

- `createTransferInterfaceInstructions`/`transferInterface` convenience path
  expects an on-curve wallet recipient for ATA derivation.
- For PDA/off-curve owners, derive destination ATA/account explicitly and use
  `transferToAccountInterface` or
  `createTransferToAccountInterfaceInstructions`.

## 4. Flow (Action Level)

### `transferInterface` (wallet recipient)

1. Validate source ATA matches sender authority + `programId`.
2. Build batched plan via `createTransferInterfaceInstructions`.
3. Split with `sliceLast` into load batches and final transfer batch.
4. Send load batches in parallel.
5. Send final transfer batch.

### `transferToAccountInterface` (explicit destination)

Same execution model, but destination account is caller-provided and not derived.

## 5. Dispatch Rules (Builder)

`createTransferToAccountInterfaceInstructions` transfer instruction dispatch:

| Condition                               | Transfer ix                                |
| --------------------------------------- | ------------------------------------------ |
| `programId` is SPL or T22 and `!wrap`   | SPL `createTransferCheckedInstruction`     |
| otherwise                               | `createLightTokenTransferCheckedInstruction` |

## 6. Concurrency Model

- Return type is `TransactionInstruction[][]`.
- `[0..n-2]` are load batches and can be sent in parallel.
- `[n-1]` is the transfer batch and must be sent after loads confirm.
- Use `sliceLast(...)` to orchestrate this pattern.
