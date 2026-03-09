# Payment Integration: `createTransferInterfaceInstructions`

Build transfer instructions for production payment flows. Returns
`TransactionInstruction[][]` with CU budgeting, sender ATA creation,
loading (decompression), and the transfer instruction. Destination token
account must exist; create it via `getOrCreateAtaInterface` or
`createAssociatedTokenAccountInterfaceIdempotentInstruction` before transfer.

## Import

```typescript
// Standard (no SPL/T22 wrapping)
import {
    createTransferInterfaceInstructions,
    getAssociatedTokenAddressInterface,
    getOrCreateAtaInterface,
    sliceLast,
} from '@lightprotocol/compressed-token';

// Unified (auto-wraps SPL/T22 to c-token ATA)
import {
    createTransferInterfaceInstructions,
    getAssociatedTokenAddressInterface,
    getOrCreateAtaInterface,
    sliceLast,
} from '@lightprotocol/compressed-token/unified';
```

## Usage

```typescript
// 1. Ensure destination exists, then build instruction batches
const destination = getAssociatedTokenAddressInterface(
    mint,
    recipient.publicKey,
);
await getOrCreateAtaInterface(rpc, payer, mint, recipient.publicKey);

const batches = await createTransferInterfaceInstructions(
    rpc,
    payer.publicKey,
    mint,
    amount,
    sender.publicKey,
    destination,
);

// 2. Customize (optional) -- append memo, priority fee, etc. to the last batch
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

## Return type

`TransactionInstruction[][]` -- an array of transaction instruction arrays.

- All batches except the last can be sent in parallel (load/decompress).
- The last batch is the transfer and must be sent after all others confirm.
- For a hot sender or <=8 cold inputs, the result is a single-element array.

Use `sliceLast(batches)` to get `{ rest, last }` for clean send orchestration.

## Options

| Option      | Default                  | Description                                            |
| ----------- | ------------------------ | ------------------------------------------------------ |
| `wrap`      | `false`                  | Include SPL/T22 wrapping to c-token ATA (unified path) |
| `programId` | `LIGHT_TOKEN_PROGRAM_ID` | Token program ID (SPL/T22/Light)                       |

## What each transaction contains

| Content                     | Load transaction | Transfer transaction |
| --------------------------- | :--------------: | :------------------: |
| `ComputeBudgetProgram`      |       yes        |         yes          |
| Sender ATA creation         | yes (idempotent) |   yes (if needed)    |
| Decompress instructions     |       yes        |   yes (if needed)    |
| Wrap SPL/T22 (unified only) |   first batch    |          --          |
| Transfer instruction        |        --        |         yes          |

## Signers

All transactions require the **payer** and the **sender** as signers.
