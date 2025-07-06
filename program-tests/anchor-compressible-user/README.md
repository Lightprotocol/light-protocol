# Anchor Compressible User Records

A comprehensive example demonstrating how to use Light Protocol's compressible SDK with Anchor framework, including the SDK helper functions for compressing and decompressing PDAs.

## Overview

This program demonstrates:

- Creating compressed user records based on the signer's public key
- Using Anchor's account constraints with compressed accounts
- Updating compressed records
- Decompressing records to regular PDAs using SDK helpers
- Compressing PDAs back to compressed accounts using SDK helpers
- Using the `PdaTimingData` trait for time-based compression controls

## Key Features

### 1. **Deterministic Addressing**

User records are created at deterministic addresses derived from:

```rust
seeds = [b"user_record", user_pubkey]
```

This ensures each user can only have one record and it can be found without scanning.

### 2. **Compressed Account Structure with Timing**

```rust
#[event]
#[derive(Clone, Debug, Default, LightHasher, LightDiscriminator)]
pub struct UserRecord {
    #[hash]
    pub owner: Pubkey,      // The user who owns this record
    pub name: String,       // User's display name
    pub bio: String,        // User's bio
    pub score: i64,         // Some score/reputation
    pub created_at: i64,    // Creation timestamp
    pub updated_at: i64,    // Last update timestamp
    // PDA timing data for compression/decompression
    pub last_written_slot: u64,
    pub slots_until_compression: u64,
}
```

### 3. **Five Main Instructions**

#### Create User Record

- Creates a new compressed account for the user
- Uses the user's pubkey as a seed for deterministic addressing
- Initializes with name, bio, timestamps, and timing data

#### Update User Record

- Updates an existing compressed user record
- Verifies ownership before allowing updates
- Can update name, bio, or increment/decrement score
- Updates the last_written_slot for timing controls

#### Decompress User Record

- Uses `decompress_idempotent` SDK helper
- Converts a compressed account to a regular on-chain PDA
- Idempotent - can be called multiple times safely
- Preserves all data during decompression

#### Compress User Record PDA

- Uses `compress_pda` SDK helper
- Compresses an existing PDA back to a compressed account
- Requires the compressed account to already exist
- Enforces timing constraints (slots_until_compression)

#### Compress User Record PDA New

- Uses `compress_pda_new` SDK helper
- Compresses a PDA into a new compressed account with a specific address
- Creates the compressed account and closes the PDA in one operation
- Also enforces timing constraints

## Integration with Light SDK Helpers

The program uses Light SDK's PDA helper functions:

1. **`decompress_idempotent`**: Safely decompresses accounts, handling the case where the PDA might already exist
2. **`compress_pda`**: Compresses an existing PDA into an existing compressed account
3. **`compress_pda_new`**: Compresses a PDA into a new compressed account with a derived address
4. **`PdaTimingData` trait**: Implements timing controls for when PDAs can be compressed

## PDA Timing Controls

The program implements the `PdaTimingData` trait to control when PDAs can be compressed:

- `last_written_slot`: Tracks when the PDA was last modified
- `slots_until_compression`: Number of slots that must pass before compression is allowed
- This prevents immediate compression after decompression, allowing for transaction finality

## Testing

The test file demonstrates:

- Creating a user record
- Updating the record
- Decompressing to a regular PDA
- Compressing back to a compressed account

Run tests with:

```bash
cd program-tests/anchor-compressible-user
cargo test-sbf
```
