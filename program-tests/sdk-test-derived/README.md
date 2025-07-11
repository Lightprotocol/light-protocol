# Native Solana Compressible Instructions Example

This example demonstrates the `add_native_compressible_instructions` macro for native Solana programs (without Anchor).

## Overview

The `add_native_compressible_instructions` macro automatically generates:

1. **Unified Data Structures**:

   - `CompressedAccountVariant` enum
   - `CompressedAccountData` struct

2. **Instruction Data Structures**:

   - `CreateCompressionConfigData`
   - `UpdateCompressionConfigData`
   - `DecompressMultiplePdasData`
   - `Compress{AccountType}Data` for each account type

3. **Instruction Processors**:

   - `process_create_compression_config`
   - `process_update_compression_config`
   - `process_decompress_multiple_pdas`
   - `process_compress_{account_type}` for each account type

4. **Utilities**:
   - `dispatch_compression_instruction` helper
   - Error types and discriminators

## Usage

### 1. Define Your Account Structures

Your account structures must implement the required traits:

```rust
#[derive(
    Clone, Debug, Default, LightHasher, LightDiscriminator, BorshDeserialize, BorshSerialize,
)]
pub struct MyPdaAccount {
    #[skip]  // Skip from hashing
    pub compression_info: CompressionInfo,
    pub data: [u8; 31],
}

impl HasCompressionInfo for MyPdaAccount {
    fn compression_info(&self) -> &CompressionInfo {
        &self.compression_info
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        &mut self.compression_info
    }
}
```

### 2. Apply the Macro

```rust
use light_sdk_macros::add_native_compressible_instructions;

#[add_native_compressible_instructions(MyPdaAccount)]
pub mod compression {
    use super::*;

    // Add any custom instruction processors here
}
```

### 3. Set Up Instruction Dispatch

```rust
pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    if instruction_data.is_empty() {
        return Err(ProgramError::InvalidInstructionData);
    }

    match instruction_data[0] {
        // Use generated compression instructions
        compression::instruction::CREATE_COMPRESSION_CONFIG
        | compression::instruction::UPDATE_COMPRESSION_CONFIG
        | compression::instruction::DECOMPRESS_MULTIPLE_PDAS
        | compression::instruction::COMPRESS_USER_RECORD => {
            compression::dispatch_compression_instruction(
                instruction_data[0],
                accounts,
                instruction_data,
            )
        }
        // Custom instructions
        10 => process_custom_instruction(accounts, &instruction_data[1..]),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}
```

## Generated Instructions

### Create Compression Config

Creates a global configuration for compression operations.

**Instruction Data**: `CreateCompressionConfigData`
**Account Layout**:

- 0: payer (signer, mut)
- 1: config PDA (mut)
- 2: program data account
- 3: authority (signer)
- 4: system program

```rust
let instruction_data = CreateCompressionConfigData {
    compression_delay: 100,
    rent_recipient: rent_recipient_pubkey,
    address_space: vec![address_tree_pubkey],
};
```

### Update Compression Config

Updates an existing compression configuration.

**Instruction Data**: `UpdateCompressionConfigData`
**Account Layout**:

- 0: config PDA (mut)
- 1: authority (signer)

### Compress Account

Compresses an existing PDA into a compressed account.

**Instruction Data**: `Compress{AccountType}Data`
**Account Layout**:

- 0: user (signer, mut)
- 1: pda_account (mut)
- 2: system_program
- 3: config PDA
- 4: rent_recipient
- 5+: Light Protocol system accounts

```rust
let compress_data = CompressMyPdaAccountData {
    proof: validity_proof,
    compressed_account_meta: compressed_meta,
};
```

### Decompress Multiple PDAs

Decompresses multiple compressed accounts into PDAs in a single transaction.

**Instruction Data**: `DecompressMultiplePdasData`
**Account Layout**:

- 0: fee_payer (signer, mut)
- 1: rent_payer (signer, mut)
- 2: system_program
- 3..system_accounts_offset: PDA accounts to decompress into
- system_accounts_offset+: Light Protocol system accounts

```rust
let decompress_data = DecompressMultiplePdasData {
    proof: validity_proof,
    compressed_accounts: vec![compressed_account_data],
    bumps: vec![pda_bump],
    system_accounts_offset: 5,
};
```

## Key Differences from Anchor Version

1. **Manual Account Validation**: No automatic account validation - you must validate accounts in your instruction processors.

2. **Raw AccountInfo Arrays**: Instead of `Context<>` structs, functions receive `&[AccountInfo]`.

3. **Manual Error Handling**: Uses `ProgramError` instead of Anchor's error types.

4. **Instruction Discriminators**: Manual discriminator constants instead of Anchor's automatic handling.

5. **Account Layout Documentation**: Must manually document expected account layouts since there's no declarative validation.

## Client-Side Usage

```typescript
// TypeScript/JavaScript client example
import { Connection, PublicKey, TransactionInstruction } from "@solana/web3.js";
import * as borsh from "borsh";

// Define instruction data schemas
const CreateCompressionConfigSchema = borsh.struct([
  borsh.u32("compression_delay"),
  borsh.publicKey("rent_recipient"),
  borsh.vec(borsh.publicKey(), "address_space"),
]);

// Build instruction
const instructionData = {
  compression_delay: 100,
  rent_recipient: rentRecipientPubkey,
  address_space: [addressTreePubkey],
};

const serialized = borsh.serialize(
  CreateCompressionConfigSchema,
  instructionData
);
const instruction = new TransactionInstruction({
  keys: [
    { pubkey: payer, isSigner: true, isWritable: true },
    { pubkey: configPda, isSigner: false, isWritable: true },
    { pubkey: programDataAccount, isSigner: false, isWritable: false },
    { pubkey: authority, isSigner: true, isWritable: false },
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
  ],
  programId: PROGRAM_ID,
  data: Buffer.concat([Buffer.from([0]), Buffer.from(serialized)]), // 0 = CREATE_COMPRESSION_CONFIG
});
```

## Best Practices

1. **Validate All Accounts**: Since there's no automatic validation, manually check all account requirements.

2. **Use Config Values**: Always load and use values from the compression config rather than hardcoding.

3. **Error Handling**: Convert all Light SDK errors to `ProgramError` for consistent error handling.

4. **Documentation**: Document account layouts clearly since they're not self-documenting like Anchor.

5. **Testing**: Test both successful and error cases thoroughly since validation is manual.

## Comparison with Manual Implementation

**Before (Manual)**:

- ~500 lines of repetitive code per account type
- Manual enum creation and trait implementations
- Easy to introduce bugs in account validation
- Inconsistent error handling

**After (Macro)**:

- ~50 lines of actual program logic
- Automatic generation of boilerplate
- Consistent validation patterns
- Standardized error handling
- Easy to add new account types

This macro provides the same convenience as the Anchor version but for native Solana programs, making compression functionality accessible without requiring the Anchor framework.
