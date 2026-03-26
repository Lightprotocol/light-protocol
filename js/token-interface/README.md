# `@lightprotocol/token-interface`

Payments-focused helpers for Light rent-free token flows.

Use this when you want SPL-style transfers with unified sender handling:
- sender side auto wraps/loads into light ATA
- recipient ATA can be light (default), SPL, or Token-2022 via `tokenProgram`

## RPC client (required)

All builders expect `createRpc()` from `@lightprotocol/stateless.js`.

```ts
import { createRpc } from '@lightprotocol/stateless.js';

// Add this to your client. It is a superset of web3.js Connection RPC plus Light APIs.
const rpc = createRpc();
// Optional: createRpc(clusterUrl)
```

## Canonical for Kit users

Use `createTransferInstructionPlan` from `/kit`.

```ts
import { createTransferInstructionPlan } from '@lightprotocol/token-interface/kit';

const transferPlan = await createTransferInstructionPlan({
    rpc,
    payer: payer.publicKey,
    mint,
    sourceOwner: sender.publicKey,
    authority: sender.publicKey,
    recipient: customer.publicKey,
    // Optional destination program:
    // tokenProgram: TOKEN_PROGRAM_ID
    amount: 25n,
});
```

If you prefer Kit instruction arrays instead of plans:

```ts
import { buildTransferInstructions } from '@lightprotocol/token-interface/kit';
```

## Canonical for web3.js users

Use `buildTransferInstructions` from the root export.

```ts
import { buildTransferInstructions } from '@lightprotocol/token-interface';

const instructions = await buildTransferInstructions({
    rpc,
    payer: payer.publicKey,
    mint,
    sourceOwner: sender.publicKey,
    authority: sender.publicKey,
    recipient: customer.publicKey,
    amount: 25n,
});

// add memo if needed, then build/sign/send transaction
```

Backwards-compatible alias:

```ts
import { createTransferInstructions } from '@lightprotocol/token-interface';
```

## Raw single-instruction helpers

Use these when you want manual orchestration:

```ts
import {
    createAtaInstruction,
    createLoadInstruction,
    createTransferCheckedInstruction,
} from '@lightprotocol/token-interface/instructions';
```

## No-wrap instruction-flow builders (advanced)

If you explicitly want to disable automatic sender wrapping, use:

```ts
import { buildTransferInstructionsNowrap } from '@lightprotocol/token-interface/instructions';
```

## Read account

```ts
import { getAta } from '@lightprotocol/token-interface';

const account = await getAta({ rpc, owner: customer.publicKey, mint });
console.log(account.amount, account.hotAmount, account.compressedAmount);
```

## Important rules

- Only one compressed sender account is loaded per call; smaller ones are ignored for that call.
- Transfer always builds checked semantics.
- Canonical builders always use wrap-enabled sender setup (`buildTransferInstructions`, `createLoadInstructions`, `createApproveInstructions`, `createRevokeInstructions`).
- If sender SPL/T22 balances were wrapped by the flow, source SPL/T22 ATAs are closed afterward.
- Recipient ATA is derived from `(recipient, mint, tokenProgram)`; default is light token program.
- Recipient-side load is still intentionally disabled.