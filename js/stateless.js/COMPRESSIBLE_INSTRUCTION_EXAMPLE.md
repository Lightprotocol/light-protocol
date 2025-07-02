# CompressibleInstruction TypeScript Implementation

This document demonstrates the TypeScript equivalent of the Rust `CompressibleInstruction` module, now organized in a clean modular structure.

## New Structure

The compressible instruction functionality is now organized in `src/compressible/`:

- **`types.ts`** - All TypeScript types and interfaces
- **`layout.ts`** - Borsh schemas and serialization functions
- **`instruction.ts`** - Standalone functions + optional class-based API
- **`index.ts`** - Clean exports and utilities

## Usage Examples

### Import Options

```typescript
// Import everything from the compressible module
import {
    // Action functions (high-level, recommended)
    initializeCompressionConfig,
    updateCompressionConfig,
    compressAccount,
    decompressAccountsIdempotent,
    // Instruction builders (low-level)
    createInitializeCompressionConfigInstruction,
    createUpdateCompressionConfigInstruction,
    createCompressAccountInstruction,
    createDecompressAccountsIdempotentInstruction,
    CompressibleInstruction,
    deriveCompressionConfigAddress,
    getProgramDataAccount,
    checkProgramUpdateAuthority,
    createCompressedAccountData,
    serializeInitializeCompressionConfigData,
    COMPRESSIBLE_DISCRIMINATORS,
} from '@lightprotocol/stateless.js/compressible';

// Or import specific items from main package
import {
    initializeCompressionConfig,
    createInitializeCompressionConfigInstruction,
    deriveCompressionConfigAddress,
    createCompressedAccountData,
    COMPRESSIBLE_DISCRIMINATORS,
} from '@lightprotocol/stateless.js';
```

### Initialize Compression Config (Action Function - Recommended)

```typescript
import { initializeCompressionConfig } from '@lightprotocol/stateless.js';
import { Rpc } from '../rpc'; // or your RPC setup

// High-level action function handles transaction building and sending
const txSignature = await initializeCompressionConfig(
    rpc,
    payer, // Signer
    programId, // PublicKey
    authority, // Signer
    compressionDelay, // number
    rentRecipient, // PublicKey
    addressSpace, // PublicKey[]
    0, // configBump (optional)
    undefined, // custom discriminator (optional)
    confirmOptions, // ConfirmOptions (optional)
);
```

### Initialize Compression Config (Instruction Builder)

```typescript
import {
    createCompressibleInitializeConfigInstruction,
    COMPRESSIBLE_DISCRIMINATORS,
} from '@lightprotocol/stateless.js';
import { PublicKey } from '@solana/web3.js';

// Using standard discriminator - standalone function (recommended)
const ix = createCompressibleInitializeConfigInstruction({
    programId,
    discriminator: COMPRESSIBLE_DISCRIMINATORS.INITIALIZE_COMPRESSION_CONFIG,
    payer: payer.publicKey,
    authority: authority.publicKey,
    compressionDelay,
    rentRecipient,
    addressSpace,
    configBump: 0,
});

// Using custom discriminator - standalone function
const customDiscriminator = [1, 2, 3, 4, 5, 6, 7, 8];
const customIx = createCompressibleInitializeConfigInstruction({
    programId,
    discriminator: customDiscriminator,
    payer: payer.publicKey,
    authority: authority.publicKey,
    compressionDelay,
    rentRecipient,
    addressSpace,
});
```

### Initialize Compression Config (Class-based API)

```typescript
import {
    CompressibleInstruction,
    COMPRESSIBLE_DISCRIMINATORS,
} from '@lightprotocol/stateless.js';

// Same functionality, class-based syntax
const ix = CompressibleInstruction.initializeCompressionConfig(
    programId,
    COMPRESSIBLE_DISCRIMINATORS.INITIALIZE_COMPRESSION_CONFIG,
    payer.publicKey,
    authority.publicKey,
    compressionDelay,
    rentRecipient,
    addressSpace,
    0, // configBump
);
```

### Update Compression Config (Action Function - Recommended)

```typescript
import { updateCompressionConfig } from '@lightprotocol/stateless.js';

// High-level action function
const txSignature = await updateCompressionConfig(
    rpc,
    payer, // Signer
    programId, // PublicKey
    authority, // Signer
    newCompressionDelay, // number | null
    newRentRecipient, // PublicKey | null
    newAddressSpace, // PublicKey[] | null
    newUpdateAuthority, // PublicKey | null
    undefined, // custom discriminator (optional)
    confirmOptions, // ConfirmOptions (optional)
);
```

### Update Compression Config (Instruction Builder)

```typescript
import {
    createUpdateCompressionConfigInstruction,
    COMPRESSIBLE_DISCRIMINATORS,
} from '@lightprotocol/stateless.js';

// Low-level instruction builder
const updateIx = createUpdateCompressionConfigInstruction(
    programId,
    COMPRESSIBLE_DISCRIMINATORS.UPDATE_COMPRESSION_CONFIG,
    authority.publicKey,
    newCompressionDelay,
    newRentRecipient,
    newAddressSpace,
    newUpdateAuthority,
);

// Class-based alternative
const updateIx2 = CompressibleInstruction.updateCompressionConfig(
    programId,
    COMPRESSIBLE_DISCRIMINATORS.UPDATE_COMPRESSION_CONFIG,
    authority.publicKey,
    newCompressionDelay,
    newRentRecipient,
    newAddressSpace,
    newUpdateAuthority,
);
```

### Compress Account

```typescript
import { createCompressAccountInstruction } from '@lightprotocol/stateless.js';

// Standalone function (recommended)
const compressIx = createCompressAccountInstruction({
    programId,
    discriminator: [1, 2, 3, 4, 5, 6, 7, 8], // custom discriminator
    payer: payer.publicKey,
    pdaToCompress,
    rentRecipient,
    compressedAccountMeta,
    validityProof,
    systemAccounts,
});
```

### Decompress Accounts Idempotent

```typescript
import * as borsh from '@coral-xyz/borsh';
import {
    createDecompressAccountsIdempotentInstruction,
    COMPRESSIBLE_DISCRIMINATORS,
} from '@lightprotocol/stateless.js';

// Define your program-specific data schema
const MyDataSchema = borsh.struct([
    borsh.u64('amount'),
    borsh.publicKey('mint'),
    // ... other fields
]);

type MyData = {
    amount: BN;
    mint: PublicKey;
    // ... other fields
};

// Standalone function (recommended)
const decompressIx = createDecompressAccountsIdempotentInstruction<MyData>({
    programId,
    discriminator: COMPRESSIBLE_DISCRIMINATORS.DECOMPRESS_ACCOUNTS_IDEMPOTENT,
    feePayer: feePayer.publicKey,
    rentPayer: rentPayer.publicKey,
    solanaAccounts,
    compressedAccountsData,
    bumps,
    validityProof,
    systemAccounts,
    dataSchema: MyDataSchema, // Required for proper serialization
});

// Class-based alternative
const decompressIx2 =
    CompressibleInstruction.decompressAccountsIdempotent<MyData>(
        programId,
        COMPRESSIBLE_DISCRIMINATORS.DECOMPRESS_ACCOUNTS_IDEMPOTENT,
        feePayer.publicKey,
        rentPayer.publicKey,
        solanaAccounts,
        compressedAccountsData,
        bumps,
        validityProof,
        systemAccounts,
        MyDataSchema,
    );
```

## Helper Utilities

### Direct Imports (Recommended)

```typescript
import {
    createCompressedAccountData,
    deriveCompressionConfigAddress,
    getProgramDataAccount,
    checkProgramUpdateAuthority,
    COMPRESSIBLE_DISCRIMINATORS,
} from '@lightprotocol/stateless.js';

// Create compressed account data
const compressedAccountData = createCompressedAccountData(
    compressedAccount,
    myDataVariant,
    seeds,
    outputStateTreeIndex,
);

// Derive compression config PDA
const [configPda, bump] = deriveCompressionConfigAddress(programId, 0);

// Get program data account for authority validation
const { programDataAddress, programDataAccountInfo } =
    await getProgramDataAccount(programId, connection);

// Check program update authority
checkProgramUpdateAuthority(programDataAccountInfo, authority);

// Access standard discriminators
const discriminators = COMPRESSIBLE_DISCRIMINATORS;
```

### Class-Based API (Alternative)

```typescript
import { CompressibleInstruction } from '@lightprotocol/stateless.js';

// Create compressed account data using class method
const compressedAccountData =
    CompressibleInstruction.createCompressedAccountData(
        compressedAccount,
        myDataVariant,
        seeds,
        outputStateTreeIndex,
    );

// Derive compression config PDA using class method
const [configPda, bump] =
    CompressibleInstruction.deriveCompressionConfigAddress(programId, 0);

// Get program data account using class method
const { programDataAddress, programDataAccountInfo } =
    await CompressibleInstruction.getProgramDataAccount(programId, connection);

// Check program update authority using class method
CompressibleInstruction.checkProgramUpdateAuthority(
    programDataAccountInfo,
    authority,
);

// Access discriminators via class constant
const discriminators = CompressibleInstruction.DISCRIMINATORS;

// Serialize config data using class method
const serializedData =
    CompressibleInstruction.serializeInitializeCompressionConfigData(
        compressionDelay,
        rentRecipient,
        addressSpace,
        configBump,
    );
```

### Complete Workflow Example (Class-Based)

```typescript
import { CompressibleInstruction } from '@lightprotocol/stateless.js';
import { Connection, PublicKey } from '@solana/web3.js';

// All utilities available through one class
const programId = new PublicKey('...');
const connection = new Connection('...');
const authority = new PublicKey('...');

// Use class constants
const discriminator =
    CompressibleInstruction.DISCRIMINATORS.INITIALIZE_COMPRESSION_CONFIG;

// Use class utilities
const [configPda, bump] =
    CompressibleInstruction.deriveCompressionConfigAddress(programId);
const { programDataAddress, programDataAccountInfo } =
    await CompressibleInstruction.getProgramDataAccount(programId, connection);

// Validate authority using class method
CompressibleInstruction.checkProgramUpdateAuthority(
    programDataAccountInfo,
    authority,
);

// Create instruction using class method
const ix = CompressibleInstruction.initializeCompressionConfig(
    programId,
    discriminator,
    payer.publicKey,
    authority,
    compressionDelay,
    rentRecipient,
    addressSpace,
    bump,
);

// Create compressed account data using class method
const compressedData = CompressibleInstruction.createCompressedAccountData(
    compressedAccount,
    myAccountData,
    seeds,
    outputStateTreeIndex,
);
```

## Type Definitions

### Core Types

```typescript
// Generic compressed account data for any program
type CompressedAccountData<T> = {
    meta: CompressedAccountMeta;
    data: T; // Program-specific variant
    seeds: Uint8Array[]; // PDA seeds without bump
};

// Instruction data for decompress idempotent
type DecompressMultipleAccountsIdempotentData<T> = {
    proof: ValidityProof;
    compressedAccounts: CompressedAccountData<T>[];
    bumps: number[];
    systemAccountsOffset: number;
};

// Update config instruction data
type UpdateCompressionConfigData = {
    newCompressionDelay: number | null;
    newRentRecipient: PublicKey | null;
    newAddressSpace: PublicKey[] | null;
    newUpdateAuthority: PublicKey | null;
};
```

### Borsh Schemas

```typescript
// Create custom schemas for your data types
export function createCompressedAccountDataSchema<T>(
    dataSchema: borsh.Layout<T>,
): borsh.Layout<CompressedAccountData<T>>;

export function createDecompressMultipleAccountsIdempotentDataSchema<T>(
    dataSchema: borsh.Layout<T>,
): borsh.Layout<DecompressMultipleAccountsIdempotentData<T>>;
```

## Key Features

1. **Clean Modular Structure**: Organized in `src/compressible/` with clear separation of concerns
2. **Dual API Design**: Both standalone functions (recommended) and class-based API
3. **Generic Type Support**: Works with any program-specific compressed account variant
4. **Custom Discriminators**: Always allows custom instruction discriminator bytes
5. **Borsh Serialization**: Uses `@coral-xyz/borsh` instead of Anchor dependency
6. **Solana SDK Patterns**: Follows patterns like `SystemProgram.transfer()`
7. **Type Safety**: Full TypeScript support with proper type checking
8. **Error Handling**: Comprehensive validation and error messages
9. **Tree Exports**: Clean imports from both main package and sub-modules

## Comparison with Rust

| Rust                                                        | TypeScript (Action)                      | TypeScript (Instruction)                             | TypeScript (Class)                                       |
| ----------------------------------------------------------- | ---------------------------------------- | ---------------------------------------------------- | -------------------------------------------------------- |
| `CompressibleInstruction::initialize_compression_config()`  | `initializeCompressionConfig(rpc, ...)`  | `createInitializeCompressionConfigInstruction(...)`  | `CompressibleInstruction.initializeCompressionConfig()`  |
| `CompressibleInstruction::update_compression_config()`      | `updateCompressionConfig(rpc, ...)`      | `createUpdateCompressionConfigInstruction(...)`      | `CompressibleInstruction.updateCompressionConfig()`      |
| `CompressibleInstruction::compress_account()`               | `compressAccount(rpc, ...)`              | `createCompressAccountInstruction(...)`              | `CompressibleInstruction.compressAccount()`              |
| `CompressibleInstruction::decompress_accounts_idempotent()` | `decompressAccountsIdempotent(rpc, ...)` | `createDecompressAccountsIdempotentInstruction(...)` | `CompressibleInstruction.decompressAccountsIdempotent()` |
| `CompressedAccountData<T>`                                  | `CompressedAccountData<T>`               | `CompressedAccountData<T>`                           | `CompressedAccountData<T>`                               |
| `ValidityProof`                                             | `ValidityProof`                          | `ValidityProof`                                      | `ValidityProof`                                          |
| `borsh::BorshSerialize`                                     | `borsh.Layout<T>`                        | `borsh.Layout<T>`                                    | `borsh.Layout<T>`                                        |

## API Philosophy

- **Action Functions**: Highest-level API. Handle RPC connection, transaction building, signing, and sending. Most convenient for applications.
- **Instruction Builders**: Mid-level API. Build individual `TransactionInstruction` objects. Good for custom transaction composition.
- **Utility Functions**: Helper functions for common operations like PDA derivation, account data creation, and authority validation.
- **Class-based API**: Complete alternative providing instruction builders, utilities, and constants through static methods. Familiar for teams migrating from other SDKs.

### Recommendation

1. **Use Action Functions** for most applications - they handle all the complexity
2. **Use Direct Utility Imports** for specific helper functions - clean and tree-shakeable
3. **Use Instruction Builders** when you need custom transaction composition or advanced control
4. **Use Class-based API** if your team prefers centralized class patterns or needs a single import

### API Styles

```typescript
// Direct imports (recommended for modern TS/JS)
import {
    initializeCompressionConfig,
    deriveCompressionConfigAddress,
} from '@lightprotocol/stateless.js';

// Class-based (alternative, all-in-one)
import { CompressibleInstruction } from '@lightprotocol/stateless.js';
const config =
    CompressibleInstruction.deriveCompressionConfigAddress(programId);
```

The TypeScript implementation provides equivalent functionality to Rust while maintaining TypeScript idioms and patterns in a clean, modular structure.
