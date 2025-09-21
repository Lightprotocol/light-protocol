# Discriminator Trait

**Path:** `program-libs/account-checks/src/discriminator.rs`

## Description

The Discriminator trait provides a type-safe system for account identification using 8-byte prefixes. This enables compile-time verification of account types and prevents account type confusion attacks.

## Trait Definition

```rust
pub const DISCRIMINATOR_LEN: usize = 8;

pub trait Discriminator {
    const LIGHT_DISCRIMINATOR: [u8; 8];
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8];

    fn discriminator() -> [u8; 8] {
        Self::LIGHT_DISCRIMINATOR
    }
}
```

## Purpose

Discriminators serve as type identifiers for Solana accounts:
- **First 8 bytes** of account data identify the account type
- **Compile-time constants** ensure type safety
- **Prevents account confusion** by validating expected types
- **Compatible with Anchor** discriminator pattern

## Implementation Pattern

### Basic Implementation
```rust
use light_account_checks::discriminator::Discriminator;

pub struct TokenAccount;

impl Discriminator for TokenAccount {
    const LIGHT_DISCRIMINATOR: [u8; 8] = [180, 4, 231, 26, 220, 144, 55, 168];
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
}
```

### With Account Data Structure
```rust
#[derive(Debug, Clone, Copy)]
pub struct ConfigAccount {
    pub discriminator: [u8; 8],  // Must match LIGHT_DISCRIMINATOR
    pub authority: [u8; 32],
    pub settings: u64,
}

impl Discriminator for ConfigAccount {
    const LIGHT_DISCRIMINATOR: [u8; 8] = [99, 111, 110, 102, 105, 103, 0, 0];  // "config\0\0"
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
}
```

## Usage with Validation Functions

### Account Initialization
```rust
use light_account_checks::checks::{account_info_init, set_discriminator};

// Initialize account with discriminator
fn initialize_config<A: AccountInfoTrait>(
    account: &A,
) -> Result<(), AccountError> {
    // Sets the discriminator in the account data
    account_info_init::<ConfigAccount, A>(account)?;

    // Or manually with mutable data
    let mut data = account.try_borrow_mut_data()?;
    set_discriminator::<ConfigAccount>(&mut data)?;

    Ok(())
}
```

### Account Validation
```rust
use light_account_checks::checks::{check_discriminator, check_account_info};

// Validate discriminator only
fn validate_discriminator(data: &[u8]) -> Result<(), AccountError> {
    check_discriminator::<ConfigAccount>(data)?;
    Ok(())
}

// Full account validation with discriminator
fn validate_config<A: AccountInfoTrait>(
    program_id: &[u8; 32],
    account: &A,
) -> Result<(), AccountError> {
    // Checks ownership AND discriminator
    check_account_info::<ConfigAccount, A>(program_id, account)?;
    Ok(())
}
```

## Discriminator Values

### Standard Patterns

1. **Sequential bytes**: Simple incrementing values
   ```rust
   const LIGHT_DISCRIMINATOR: [u8; 8] = [180, 4, 231, 26, 220, 144, 55, 168];
   ```

2. **ASCII strings**: Human-readable identifiers
   ```rust
   const LIGHT_DISCRIMINATOR: [u8; 8] = *b"tokenacc";  // "tokenacc"
   const LIGHT_DISCRIMINATOR: [u8; 8] = *b"config\0\0"; // "config" with padding
   ```

3. **Hash-derived**: From account type name
   ```rust
   // First 8 bytes of sha256("ConfigAccount")
   const LIGHT_DISCRIMINATOR: [u8; 8] = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0];
   ```

4. **Anchor-compatible**: Matching Anchor's discriminator
   ```rust
   // Anchor uses sha256("account:<AccountName>")[..8]
   const LIGHT_DISCRIMINATOR: [u8; 8] = anchor_discriminator();
   ```

## Integration with Checks Module

The checks module provides functions that work with Discriminator types:

| Function | Purpose | Discriminator Usage |
|----------|---------|-------------------|
| `set_discriminator<T>` | Initialize account | Writes `T::LIGHT_DISCRIMINATOR` |
| `check_discriminator<T>` | Validate type | Compares against `T::LIGHT_DISCRIMINATOR` |
| `account_info_init<T>` | Initialize with type | Sets discriminator for type T |
| `check_account_info<T>` | Full validation | Checks discriminator matches T |
| `check_account_info_mut<T>` | Validate mutable | Includes discriminator check |
| `check_account_info_non_mut<T>` | Validate readonly | Includes discriminator check |

## Error Handling

Discriminator validation can return these errors:

- **InvalidDiscriminator (12006)**: Mismatch between expected and actual
- **InvalidAccountSize (12010)**: Account smaller than 8 bytes
- **AlreadyInitialized (12012)**: Non-zero discriminator when initializing
- **BorrowAccountDataFailed (12009)**: Can't access account data

## Best Practices

1. **Use unique discriminators**: Avoid collisions between account types
   ```rust
   // BAD: Same discriminator for different types
   impl Discriminator for AccountA {
       const LIGHT_DISCRIMINATOR: [u8; 8] = [1, 0, 0, 0, 0, 0, 0, 0];
   }
   impl Discriminator for AccountB {
       const LIGHT_DISCRIMINATOR: [u8; 8] = [1, 0, 0, 0, 0, 0, 0, 0];  // Collision!
   }
   ```

2. **Document discriminator values**: Make values discoverable
   ```rust
   /// Discriminator: [99, 111, 110, 102, 105, 103, 0, 0] ("config\0\0")
   impl Discriminator for ConfigAccount {
       const LIGHT_DISCRIMINATOR: [u8; 8] = *b"config\0\0";
       const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
   }
   ```

3. **Validate before deserialization**: Check discriminator first
   ```rust
   fn deserialize_config(data: &[u8]) -> Result<ConfigAccount, AccountError> {
       // Check discriminator BEFORE deserializing
       check_discriminator::<ConfigAccount>(data)?;

       // Safe to deserialize after validation
       let config = ConfigAccount::deserialize(data)?;
       Ok(config)
   }
   ```

## Example: Multi-Account System

```rust
// Define discriminators for different account types
pub struct UserAccount;
impl Discriminator for UserAccount {
    const LIGHT_DISCRIMINATOR: [u8; 8] = *b"user\0\0\0\0";
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
}

pub struct AdminAccount;
impl Discriminator for AdminAccount {
    const LIGHT_DISCRIMINATOR: [u8; 8] = *b"admin\0\0\0";
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
}

// Process instruction with type validation
fn process_instruction<A: AccountInfoTrait>(
    program_id: &[u8; 32],
    accounts: &[A],
) -> Result<(), AccountError> {
    let user = &accounts[0];
    let admin = &accounts[1];

    // Validate each account has correct type
    check_account_info::<UserAccount, A>(program_id, user)?;
    check_account_info::<AdminAccount, A>(program_id, admin)?;

    // Safe to proceed - types are verified
    Ok(())
}
```

## See Also
- [ACCOUNT_CHECKS.md](ACCOUNT_CHECKS.md) - Validation functions using discriminators
- [ERRORS.md](ERRORS.md) - Error codes for discriminator failures
