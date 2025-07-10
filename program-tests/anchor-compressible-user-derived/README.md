# Example: Using the add_compressible_instructions Macro

This example shows how to use the `add_compressible_instructions` macro to automatically generate compression-related instructions for your Anchor program.

## Basic Setup

```rust
use anchor_lang::prelude::*;
use light_sdk::{
    compressible::{CompressionInfo, HasCompressionInfo},
    derive_light_cpi_signer, LightDiscriminator, LightHasher,
};
use light_sdk_macros::add_compressible_instructions;

declare_id!("YourProgramId11111111111111111111111111111");

// Define your CPI signer
pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("YourCpiSignerPubkey11111111111111111111111");

// Apply the macro to your program module
#[add_compressible_instructions(UserRecord, GameSession)]
#[program]
pub mod my_program {
    use super::*;

    // The macro automatically generates these instructions:
    // - create_compression_config (config management)
    // - update_compression_config (config management)
    // - compress_user_record (compress existing PDA)
    // - compress_game_session (compress existing PDA)
    // - decompress_multiple_pdas (decompress compressed accounts)
    //
    // NOTE: create_user_record and create_game_session are NOT generated
    // because they typically need custom initialization logic

    // You can still add your own custom instructions here
}
```

## Define Your Account Structures

```rust
#[derive(Debug, LightHasher, LightDiscriminator, Default)]
#[account]
pub struct UserRecord {
    #[skip]  // Skip compression_info from hashing
    pub compression_info: CompressionInfo,
    #[hash]  // Include in hash
    pub owner: Pubkey,
    #[hash]
    pub name: String,
    pub score: u64,
}

// Implement the required trait
impl HasCompressionInfo for UserRecord {
    fn compression_info(&self) -> &CompressionInfo {
        &self.compression_info
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        &mut self.compression_info
    }
}
```

## Generated Instructions

### 1. Config Management

```typescript
// Create config (only program upgrade authority can call)
await program.methods
  .createCompressibleConfig(
    100, // compression_delay
    rentRecipient,
    addressSpace
  )
  .accounts({
    payer: wallet.publicKey,
    config: configPda,
    programData: programDataPda,
    authority: upgradeAuthority,
    systemProgram: SystemProgram.programId,
  })
  .signers([upgradeAuthority])
  .rpc();

// Update config
await program.methods
  .updateCompressibleConfig(
    200, // new_compression_delay (optional)
    newRentRecipient, // (optional)
    newAddressSpace, // (optional)
    newUpdateAuthority // (optional)
  )
  .accounts({
    config: configPda,
    authority: configUpdateAuthority,
  })
  .signers([configUpdateAuthority])
  .rpc();
```

### 2. Compress Existing PDA

```typescript
await program.methods
  .compressUserRecord(proof, compressedAccountMeta)
  .accounts({
    user: user.publicKey,
    pdaAccount: userRecordPda,
    systemProgram: SystemProgram.programId,
    config: configPda,
    rentRecipient: rentRecipient,
  })
  .remainingAccounts(lightSystemAccounts)
  .signers([user])
  .rpc();
```

### 3. Decompress Multiple PDAs

```typescript
const compressedAccounts = [
  {
    meta: compressedAccountMeta1,
    data: { userRecord: userData },
    seeds: [Buffer.from("user_record"), user.publicKey.toBuffer()],
  },
  {
    meta: compressedAccountMeta2,
    data: { gameSession: gameData },
    seeds: [
      Buffer.from("game_session"),
      sessionId.toArrayLike(Buffer, "le", 8),
    ],
  },
];

await program.methods
  .decompressMultiplePdas(
    proof,
    compressedAccounts,
    [userBump, gameBump], // PDA bumps
    systemAccountsOffset
  )
  .accounts({
    feePayer: payer.publicKey,
    rentPayer: payer.publicKey,
    systemProgram: SystemProgram.programId,
  })
  .remainingAccounts([
    ...pdaAccounts, // PDAs to decompress into
    ...lightSystemAccounts, // Light Protocol system accounts
  ])
  .signers([payer])
  .rpc();
```

## What You Need to Implement

Since the macro only generates compression-related instructions, you need to implement:

### 1. Create Instructions

Implement your own create instructions for each account type:

```rust
#[derive(Accounts)]
pub struct CreateUserRecord<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init,
        payer = user,
        space = 8 + UserRecord::INIT_SPACE,
        seeds = [b"user_record", user.key().as_ref()],
        bump,
    )]
    pub user_record: Account<'info, UserRecord>,
    pub system_program: Program<'info, System>,
}

pub fn create_user_record(
    ctx: Context<CreateUserRecord>,
    name: String,
) -> Result<()> {
    let user_record = &mut ctx.accounts.user_record;
    
    // Your custom initialization logic here
    user_record.compression_info = CompressionInfo::new()?;
    user_record.owner = ctx.accounts.user.key();
    user_record.name = name;
    user_record.score = 0;
    
    Ok(())
}
```

### 2. Update Instructions

Implement update instructions for your account types with your custom business logic.

## Customization

### Custom Seeds

Use custom seeds in your PDA derivation and pass them in the `seeds` parameter when decompressing:

```rust
seeds = [b"custom_prefix", user.key().as_ref(), &session_id.to_le_bytes()]
```

## Best Practices

1. **Create Config Early**: Create the config immediately after program deployment
2. **Use Config Values**: Always use config values instead of hardcoded constants
3. **Validate Rent Recipient**: The macro automatically validates rent recipient matches config
4. **Handle Compression Timing**: Respect the compression delay from config
5. **Batch Operations**: Use decompress_multiple_pdas for efficiency

## Migration from Manual Implementation

If migrating from a manual implementation:

1. Update your account structs to use `CompressionInfo` instead of separate fields
2. Implement the `HasCompressionInfo` trait
3. Replace your manual instructions with the macro
4. Update client code to use the new instruction names
