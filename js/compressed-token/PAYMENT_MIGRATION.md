# SPL Token to CToken Payment Migration

Mirrors SPL Token's API. Same pattern, same flow.

## TL;DR

```typescript
// SPL Token
import { transfer, getOrCreateAssociatedTokenAccount } from '@solana/spl-token';

// CToken
import {
    transferInterface,
    getOrCreateAtaInterface,
} from '@lightprotocol/compressed-token';
```

## Action Level

### SPL Token

```typescript
const recipientAta = await getOrCreateAssociatedTokenAccount(
    connection,
    payer,
    mint,
    recipient,
);
await transfer(
    connection,
    payer,
    sourceAta,
    recipientAta.address,
    owner,
    amount,
);
```

### CToken

```typescript
const recipientAta = await getOrCreateAtaInterface(rpc, payer, mint, recipient);
await transferInterface(
    rpc,
    payer,
    sourceAta,
    recipientAta.address,
    owner,
    mint,
    amount,
);
```

Same two-step pattern. `transferInterface` auto-loads sender's unified balance (cold + SPL + T22).

---

## Instruction Level

### SPL Token

```typescript
import {
    createAssociatedTokenAccountIdempotentInstruction,
    createTransferInstruction,
    getAssociatedTokenAddressSync,
} from '@solana/spl-token';

const sourceAta = getAssociatedTokenAddressSync(mint, sender);
const recipientAta = getAssociatedTokenAddressSync(mint, recipient);

const tx = new Transaction().add(
    createAssociatedTokenAccountIdempotentInstruction(
        payer,
        recipientAta,
        recipient,
        mint,
    ),
    createTransferInstruction(sourceAta, recipientAta, sender, amount),
);
```

### CToken (sender already hot)

```typescript
import {
    getAtaAddressInterface,
    createAtaInterfaceIdempotentInstruction,
    createTransferInterfaceInstruction,
    CTOKEN_PROGRAM_ID,
} from '@lightprotocol/compressed-token';

const sourceAta = getAtaAddressInterface(mint, sender);
const recipientAta = getAtaAddressInterface(mint, recipient);

const tx = new Transaction().add(
    createAtaInterfaceIdempotentInstruction(
        payer,
        recipientAta,
        recipient,
        mint,
        CTOKEN_PROGRAM_ID,
    ),
    createTransferInterfaceInstruction(sourceAta, recipientAta, sender, amount),
);
```

### CToken (sender may be cold - needs loading)

```typescript
import {
    loadAtaInstructions,
    getAtaAddressInterface,
    createAtaInterfaceIdempotentInstruction,
    createTransferInterfaceInstruction,
    CTOKEN_PROGRAM_ID,
} from '@lightprotocol/compressed-token';

// 1. Derive addresses
const sourceAta = getAtaAddressInterface(mint, sender);
const recipientAta = getAtaAddressInterface(mint, recipient);

// 2. Build load instructions (empty if already hot)
const loadIxs = await loadAtaInstructions(rpc, payer, sourceAta, sender, mint);

// 3. Build transaction
const tx = new Transaction().add(
    ...loadIxs, // Load sender if cold (wrap SPL/T22, decompress)
    createAtaInterfaceIdempotentInstruction(
        payer,
        recipientAta,
        recipient,
        mint,
        CTOKEN_PROGRAM_ID,
    ),
    createTransferInterfaceInstruction(sourceAta, recipientAta, sender, amount),
);
```

### CToken (sender pre-fetched)

```typescript
import {
    getAtaInterface,
    loadAtaInstructionsFromInterface,
    getAtaAddressInterface,
    createAtaInterfaceIdempotentInstruction,
    createTransferInterfaceInstruction,
    CTOKEN_PROGRAM_ID,
} from '@lightprotocol/compressed-token';

// 1. Pre-fetch sender's unified balance
const senderAtaInfo = await getAtaInterface(rpc, sender, mint);

// 2. Build load instructions from interface (empty if already hot)
const loadIxs = await loadAtaInstructionsFromInterface(
    rpc,
    payer,
    senderAtaInfo,
);

// 3. Derive addresses
const sourceAta = getAtaAddressInterface(mint, sender);
const recipientAta = getAtaAddressInterface(mint, recipient);

// 4. Build transaction
const tx = new Transaction().add(
    ...loadIxs,
    createAtaInterfaceIdempotentInstruction(
        payer,
        recipientAta,
        recipient,
        mint,
        CTOKEN_PROGRAM_ID,
    ),
    createTransferInterfaceInstruction(sourceAta, recipientAta, sender, amount),
);
```

---

## Instruction Mapping

| SPL Token                                           | CToken                                                      |
| --------------------------------------------------- | ----------------------------------------------------------- |
| `getAssociatedTokenAddressSync`                     | `getAtaAddressInterface`                                    |
| `createAssociatedTokenAccountIdempotentInstruction` | `createAtaInterfaceIdempotentInstruction`                   |
| `createTransferInstruction`                         | `createTransferInterfaceInstruction`                        |
| N/A                                                 | `loadAtaInstructions` (fetch + build)                       |
| N/A                                                 | `loadAtaInstructionsFromInterface` (build from pre-fetched) |

---

## Key Differences

|                     | SPL Token              | CToken                                  |
| ------------------- | ---------------------- | --------------------------------------- |
| Recipient ATA       | Create before transfer | Create before transfer                  |
| Sender balance      | Single ATA             | Unified (cold + SPL + T22 + hot)        |
| Loading             | N/A                    | `loadAtaInstructions` or auto in action |
| `destination` param | ATA address            | ATA address                             |

---

## Common Patterns

### Check if loading needed

```typescript
const ata = await getAtaInterface(rpc, owner, mint);
if (ata.isCold) {
    // Need to include load instructions
}
```

### Get unified balance

```typescript
const ata = await getAtaInterface(rpc, owner, mint);
const totalBalance = ata.parsed.amount; // All sources combined
```

### Idempotent recipient ATA

Always safe to include - no-op if exists:

```typescript
createAtaInterfaceIdempotentInstruction(
    payer,
    recipientAta,
    recipient,
    mint,
    CTOKEN_PROGRAM_ID,
);
```
