# Using c-token for Payments

**TL;DR**: Same API patterns, 1/200th ATA creation cost. Your users get the same USDC, just stored more efficiently.

---

## Setup

```typescript
import { createRpc } from "@lightprotocol/stateless.js";

import {
  getOrCreateAtaInterface,
  getAtaInterface,
  getAssociatedTokenAddressInterface,
  transferInterface,
  unwrap,
} from "@lightprotocol/compressed-token/unified";

const rpc = createRpc(RPC_ENDPOINT);
```

---

## 1. Receive Payments

**SPL Token:**

```typescript
import { getOrCreateAssociatedTokenAccount } from "@solana/spl-token";

const ata = await getOrCreateAssociatedTokenAccount(
  connection,
  payer,
  mint,
  recipient
);
// Share ata.address with sender

console.log(ata.amount);
```

**SPL Token (instruction-level):**

```typescript
import {
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountIdempotentInstruction,
} from "@solana/spl-token";

const ata = getAssociatedTokenAddressSync(mint, recipient);

const tx = new Transaction().add(
  createAssociatedTokenAccountIdempotentInstruction(
    payer.publicKey,
    ata,
    recipient,
    mint
  )
);
```

**c-token:**

```typescript
const ata = await getOrCreateAtaInterface(rpc, payer, mint, recipient);
// Share ata.parsed.address with sender

console.log(ata.parsed.amount);
```

**c-token (instruction-level):**

```typescript
import {
  createAssociatedTokenAccountInterfaceIdempotentInstruction,
  getAssociatedTokenAddressInterface,
} from "@lightprotocol/compressed-token/unified";
import { CTOKEN_PROGRAM_ID } from "@lightprotocol/stateless.js";

const ata = getAssociatedTokenAddressInterface(mint, recipient);

const tx = new Transaction().add(
  createAssociatedTokenAccountInterfaceIdempotentInstruction(
    payer.publicKey,
    ata,
    recipient,
    mint,
    CTOKEN_PROGRAM_ID
  )
);
```

---

## 2. Send Payments

**SPL Token:**

```typescript
import { transfer } from "@solana/spl-token";
const sourceAta = getAssociatedTokenAddressSync(mint, owner.publicKey);
const destinationAta = getAssociatedTokenAddressSync(mint, recipient);

await transfer(
  connection,
  payer,
  sourceAta,
  destinationAta,
  owner,
  amount,
  decimals
);
```

**SPL Token (instruction-level):**

```typescript
import {
  getAssociatedTokenAddressSync,
  createTransferInstruction,
} from "@solana/spl-token";

const sourceAta = getAssociatedTokenAddressSync(mint, owner.publicKey);
const destinationAta = getAssociatedTokenAddressSync(mint, recipient);

const tx = new Transaction().add(
  createTransferInstruction(sourceAta, destinationAta, owner.publicKey, amount)
);
```

**c-token:**

```typescript
const sourceAta = getAssociatedTokenAddressInterface(mint, owner.publicKey);
const destinationAta = getAssociatedTokenAddressInterface(mint, recipient);

await transferInterface(
  rpc,
  payer,
  sourceAta,
  mint,
  destinationAta,
  owner,
  amount
);
```

**c-token (instruction-level):**

```typescript
import {
  createLoadAtaInstructions,
  createTransferInterfaceInstruction,
  getAssociatedTokenAddressInterface,
} from "@lightprotocol/compressed-token/unified";

const sourceAta = getAssociatedTokenAddressInterface(mint, owner.publicKey);
const destinationAta = getAssociatedTokenAddressInterface(mint, recipient);

const tx = new Transaction().add(
  ...(await createLoadAtaInstructions(
    rpc,
    sourceAta,
    owner.publicKey,
    mint,
    payer.publicKey
  )),
  createTransferInterfaceInstruction(
    sourceAta,
    destinationAta,
    owner.publicKey,
    amount
  )
);
```

To ensure your recipient's ATA exists you can prepend an idempotent creation instruction in the same atomic transaction:

**SPL Token:**

```typescript
import {
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountIdempotentInstruction,
} from "@solana/spl-token";

const destinationAta = getAssociatedTokenAddressSync(mint, recipient);
const createAtaIx = createAssociatedTokenAccountIdempotentInstruction(
  payer.publicKey,
  destinationAta,
  recipient,
  mint
);

new Transaction().add(createAtaIx, transferIx);
```

**c-token:**

```typescript
import {
  getAssociatedTokenAddressInterface,
  createAssociatedTokenAccountInterfaceIdempotentInstruction,
} from "@lightprotocol/compressed-token/unified";
import { CTOKEN_PROGRAM_ID } from "@lightprotocol/stateless.js";

const destinationAta = getAssociatedTokenAddressInterface(mint, recipient);
const createAtaIx = createAssociatedTokenAccountInterfaceIdempotentInstruction(
  payer.publicKey,
  destinationAta,
  recipient,
  mint,
  CTOKEN_PROGRAM_ID
);

new Transaction().add(createAtaIx, transferIx);
```

---

## 3. Show Balance

**SPL Token:**

```typescript
import { getAccount } from "@solana/spl-token";

const account = await getAccount(connection, ata);
console.log(account.amount);
```

**c-token:**

```typescript
const ata = getAssociatedTokenAddressInterface(mint, owner);
const account = await getAtaInterface(rpc, ata, owner, mint);

console.log(account.parsed.amount);
```

---

## 4. Transaction History

**SPL Token:**

```typescript
const signatures = await connection.getSignaturesForAddress(ata);
```

**c-token:**

```typescript
// Unified: fetches both on-chain and compressed tx signatures
const result = await rpc.getSignaturesForOwnerInterface(owner);

console.log(result.signatures); // Merged + deduplicated
console.log(result.solana); // On-chain txs only
console.log(result.compressed); // Compressed txs only
```

Use `getSignaturesForAddressInterface(address)` if you want address-specific rather than owner-wide history.

---

## 5. Unwrap to SPL

When users need vanilla SPL tokens (eg., for CEX off-ramp):

**c-token -> SPL ATA:**

```typescript
import { getAssociatedTokenAddressSync } from "@solana/spl-token";

// SPL ATA must exist
const splAta = getAssociatedTokenAddressSync(mint, owner.publicKey);

await unwrap(rpc, payer, owner, mint, splAta, amount);
```

**c-token (instruction-level):**

```typescript
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import {
  createLoadAtaInstructions,
  createUnwrapInstruction,
  getAssociatedTokenAddressInterface,
} from "@lightprotocol/compressed-token/unified";
import { getSplInterfaceInfos } from "@lightprotocol/compressed-token";

const ctokenAta = getAssociatedTokenAddressInterface(mint, owner.publicKey);
const splAta = getAssociatedTokenAddressSync(mint, owner.publicKey);

const splInterfaceInfos = await getSplInterfaceInfos(rpc, mint);
const splInterfaceInfo = splInterfaceInfos.find((i) => i.isInitialized);

const tx = new Transaction().add(
  ...(await createLoadAtaInstructions(
    rpc,
    ctokenAta,
    owner.publicKey,
    mint,
    payer.publicKey
  )),
  createUnwrapInstruction(
    ctokenAta,
    splAta,
    owner.publicKey,
    mint,
    amount,
    splInterfaceInfo
  )
);
```

---

## Quick Reference

| Operation      | SPL Token                             | c-token (unified)                      |
| -------------- | ------------------------------------- | -------------------------------------- |
| Get/Create ATA | `getOrCreateAssociatedTokenAccount()` | `getOrCreateAtaInterface()`            |
| Derive ATA     | `getAssociatedTokenAddress()`         | `getAssociatedTokenAddressInterface()` |
| Transfer       | `transferChecked()`                   | `transferInterface()`                  |
| Get Balance    | `getAccount()`                        | `getAtaInterface()`                    |
| Tx History     | `getSignaturesForAddress()`           | `rpc.getSignaturesForOwnerInterface()` |
| Exit to SPL    | N/A                                   | `unwrap()`                             |

---

Need help with integration? Reach out: [support@lightprotocol.com](mailto:support@lightprotocol.com)
