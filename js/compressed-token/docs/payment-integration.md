# Payment Integration: `createTransferInterfaceInstructions`

Build transfer instructions for production payment flows. Returns
`TransactionInstruction[][]` with load/decompression batches followed by the
final transfer batch.

`createTransferInterfaceInstructions` now takes a **recipient wallet** and
derives/ensures destination ATA internally.

If you need an explicit destination token account (program-owned/custom), use
`createTransferToAccountInterfaceInstructions`.

## Import

```typescript
// Standard (no SPL/T22 wrapping; decimals required)
import {
    createTransferInterfaceInstructions,
    createTransferToAccountInterfaceInstructions,
    getMintInterface,
    sliceLast,
} from '@lightprotocol/compressed-token';

// Unified (auto-wraps SPL/T22 to c-token ATA; decimals resolved internally)
import {
    createTransferInterfaceInstructions,
    createTransferToAccountInterfaceInstructions,
    sliceLast,
} from '@lightprotocol/compressed-token/unified';
```

## Usage

```typescript
// 1. Build instruction batches (wallet-recipient path)
// Standard path: decimals is required. Unified path resolves decimals internally.
const decimals = (await getMintInterface(rpc, mint)).mint.decimals;
const batches = await createTransferInterfaceInstructions(
    rpc,
    payer.publicKey,
    mint,
    amount,
    sender.publicKey,
    recipient.publicKey,
    decimals, // omit when using unified import
);

// 2. Customize (optional) -- append memo/priority fee to the final batch
batches.at(-1)!.push(memoIx);

// 3. Build all transactions
const { blockhash } = await rpc.getLatestBlockhash();
const txns = batches.map(ixs => buildTx(ixs, blockhash, payer));

// 4. Sign all at once (one wallet prompt)
const signed = await wallet.signAllTransactions(txns);

// 5. Send: loads in parallel, then transfer
const { rest, last } = sliceLast(signed);
await Promise.all(rest.map(tx => send(tx)));
await send(last);
```

## Explicit destination-account variant

```typescript
const destinationTokenAccount = /* PDA or program-owned token account */;
const decimals = (await getMintInterface(rpc, mint)).mint.decimals;

const batches = await createTransferToAccountInterfaceInstructions(
    rpc,
    payer.publicKey,
    mint,
    amount,
    sender.publicKey,
    destinationTokenAccount,
    decimals,
);
```

## Return type

`TransactionInstruction[][]` -- an array of transaction instruction arrays.

- All batches except the last can be sent in parallel (load/decompress).
- The last batch is the transfer and must be sent after all others confirm.
- For a hot sender or <=8 cold inputs, the result is a single-element array.

Use `sliceLast(batches)` to get `{ rest, last }` for clean send orchestration.

## Options

| Option      | Default                  | Description                                                                 |
| ----------- | ------------------------ | --------------------------------------------------------------------------- |
| `wrap`      | `false` (standard only)  | Standard path: opt-in. Unified path does not expose `wrap` (forced `true`). |
| `programId` | `LIGHT_TOKEN_PROGRAM_ID` | Token program ID (SPL/T22/Light)                                            |

## What each transaction contains

| Content                     |                                 Load transaction                                  | Transfer transaction |
| --------------------------- | :-------------------------------------------------------------------------------: | :------------------: |
| `ComputeBudgetProgram`      |                                        yes                                        |         yes          |
| Sender (owner) ATA creation |                                 yes (idempotent)                                  |   yes (if needed)    |
| Recipient ATA creation      |                                        --                                         |         yes          |
| Decompress instructions     |                                        yes                                        |   yes (if needed)    |
| Wrap SPL/T22 (unified only) | first load batch (when multiple batches); single batch = load + transfer together |          --          |
| Transfer instruction        |                                        --                                         |         yes          |

## Signers

All transactions require the **payer** and the **sender** (token account owner or delegate) as signers.
