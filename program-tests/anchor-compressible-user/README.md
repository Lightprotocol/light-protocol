# Simple Anchor User Records Template

A basic Anchor program template demonstrating a simple user record system with create and update functionality.

## Overview

This is a minimal template showing:

- Creating user records as PDAs (Program Derived Addresses)
- Updating existing user records
- Basic ownership validation

## Account Structure

```rust
#[account]
pub struct UserRecord {
    pub owner: Pubkey,    // The user who owns this record
    pub name: String,     // User's name
    pub score: u64,       // User's score
}
```

## Instructions

### Create Record

- Creates a new user record PDA
- Seeds: `[b"user_record", user_pubkey]`
- Initializes with name and score of 0

### Update Record

- Updates an existing user record
- Validates ownership before allowing updates
- Can update both name and score

## Usage

```bash
# Build
anchor build

# Test
anchor test
```

## PDA Derivation

User records are stored at deterministic addresses:

```rust
let (user_record_pda, bump) = Pubkey::find_program_address(
    &[b"user_record", user.key().as_ref()],
    &program_id,
);
```

This ensures each user can only have one record.
