# AccountIterator

**Path:** `program-libs/account-checks/src/account_iterator.rs`

## Description

AccountIterator provides sequential account processing with enhanced error reporting. When accounts are missing or validation fails, it reports the exact location (file:line:column) where the error occurred, making debugging significantly easier in complex instruction processing.

All methods are marked with `#[inline(always)]` for performance optimization and `#[track_caller]` for accurate error location reporting.

## Core Structure

```rust
pub struct AccountIterator<'info, T: AccountInfoTrait> {
    accounts: &'info [T],
    position: usize,
    owner: [u8; 32],  // Reserved for future use
}
```

## Constructor Methods

### `new`
```rust
fn new(accounts: &'info [T]) -> Self
```
Basic constructor for general use.

### `new_with_owner`
```rust
fn new_with_owner(accounts: &'info [T], owner: [u8; 32]) -> Self
```
Constructor that stores owner for future validation extensions (currently unused).

## Account Retrieval Methods

### Basic Retrieval

#### `next_account`
```rust
fn next_account(&mut self, account_name: &str) -> Result<&'info T, AccountError>
```
- Gets next account with descriptive name for error messages
- **Error:** `NotEnoughAccountKeys` (20014) with detailed location

### Validated Retrieval

#### `next_signer`
```rust
fn next_signer(&mut self, account_name: &str) -> Result<&'info T, AccountError>
```
- Gets next account and validates it's a signer
- **Errors:**
  - `NotEnoughAccountKeys` (20014)
  - `InvalidSigner` (20009)

#### `next_signer_mut`
```rust
fn next_signer_mut(&mut self, account_name: &str) -> Result<&'info T, AccountError>
```
- Gets next account validating signer AND writable
- **Errors:**
  - `NotEnoughAccountKeys` (20014)
  - `InvalidSigner` (20009)
  - `AccountNotMutable` (20002)

#### `next_mut`
```rust
fn next_mut(&mut self, account_name: &str) -> Result<&'info T, AccountError>
```
- Gets next account validating it's writable
- **Errors:**
  - `NotEnoughAccountKeys` (20014)
  - `AccountNotMutable` (20002)

#### `next_non_mut`
```rust
fn next_non_mut(&mut self, account_name: &str) -> Result<&'info T, AccountError>
```
- Gets next account validating it's NOT writable
- **Errors:**
  - `NotEnoughAccountKeys` (20014)
  - `AccountMutable` (20005)

### Special Retrieval

#### `next_checked_pubkey`
```rust
fn next_checked_pubkey(
    &mut self,
    account_name: &str,
    pubkey: [u8; 32]
) -> Result<&'info T, AccountError>
```
- Gets next account and validates its public key matches
- **Errors:**
  - `NotEnoughAccountKeys` (20014)
  - `InvalidAccount` (20015) with expected/actual keys

#### `next_option`
```rust
fn next_option(
    &mut self,
    account_name: &str,
    is_some: bool
) -> Result<Option<&'info T>, AccountError>
```
- Conditionally gets next account based on `is_some` flag
- Returns `None` if `is_some` is false (doesn't advance iterator)

#### `next_option_mut`
```rust
fn next_option_mut(
    &mut self,
    account_name: &str,
    is_some: bool
) -> Result<Option<&'info T>, AccountError>
```
- Like `next_option` but validates writable if present

## Bulk Access Methods

### `remaining`
```rust
fn remaining(self) -> Result<&'info [T], AccountError>
```
- Returns all unprocessed accounts
- **Consumes the iterator** - cannot use iterator after calling this method
- **Error:** `NotEnoughAccountKeys` (20014) if iterator exhausted
- Use case: Getting all remaining accounts for dynamic processing

### `remaining_unchecked`
```rust
fn remaining_unchecked(self) -> Result<&'info [T], AccountError>
```
- Returns remaining accounts or empty slice if exhausted
- **Consumes the iterator** - cannot use iterator after calling this method
- Never errors - returns empty slice if no accounts remaining
- Use case: Optional remaining accounts where empty is acceptable

## Status Methods

- `position()` - Current index in account array
- `len()` - Total number of accounts
- `is_empty()` - Whether account array is empty
- `iterator_is_empty()` - Whether all accounts have been processed

## Usage Examples

### Basic Instruction Processing
```rust
use light_account_checks::{AccountIterator, AccountInfoTrait, AccountError};

fn process_transfer<A: AccountInfoTrait>(
    accounts: &[A],
) -> Result<(), AccountError> {
    let mut iter = AccountIterator::new(accounts);

    let authority = iter.next_signer("authority")?;
    let source = iter.next_mut("source_account")?;
    let destination = iter.next_mut("destination_account")?;
    let mint = iter.next_non_mut("mint")?;

    // Process transfer...
    Ok(())
}
```

### Optional Accounts
```rust
fn process_transfer_with_fee<A: AccountInfoTrait>(
    accounts: &[A],
    collect_fee: bool,
) -> Result<(), AccountError> {
    let mut iter = AccountIterator::new(accounts);

    let authority = iter.next_signer("authority")?;
    let source = iter.next_mut("source")?;
    let destination = iter.next_mut("destination")?;

    // Fee account only if collect_fee is true
    let fee_account = iter.next_option_mut("fee_account", collect_fee)?;

    if let Some(fee_acc) = fee_account {
        // Process fee collection
    }

    Ok(())
}
```

### System Program Validation
```rust
fn process_with_system_program<A: AccountInfoTrait>(
    accounts: &[A],
) -> Result<(), AccountError> {
    let mut iter = AccountIterator::new(accounts);

    let payer = iter.next_signer_mut("payer")?;
    let new_account = iter.next_mut("new_account")?;

    // Validate system program
    let system_program = iter.next_checked_pubkey(
        "system_program",
        solana_program::system_program::ID.to_bytes()
    )?;

    Ok(())
}
```

### Processing Variable Account Lists
```rust
fn process_multiple_transfers<A: AccountInfoTrait>(
    accounts: &[A],
) -> Result<(), AccountError> {
    let mut iter = AccountIterator::new(accounts);

    let authority = iter.next_signer("authority")?;
    let source = iter.next_mut("source")?;

    // Get all remaining destination accounts
    let destinations = iter.remaining()?;

    for (i, dest) in destinations.iter().enumerate() {
        // Validate each destination
        check_mut(dest).map_err(|_| {
            solana_msg::msg!("Destination {} not writable", i);
            AccountError::AccountNotMutable
        })?;
    }

    Ok(())
}
```

## Error Messages

AccountIterator provides detailed error messages with location tracking:

```
ERROR: Not enough accounts. Requested 'mint' at index 3 but only 2 accounts available. src/processor.rs:45:12

ERROR: Invalid Signer. for account 'authority' at index 0  src/processor.rs:42:8

ERROR: Invalid Account. for account 'system_program' address: 11111111111111111111111111111112, expected: 11111111111111111111111111111111, at index 4  src/processor.rs:48:15
```

Note: The `#[track_caller]` attribute on methods enables accurate file:line:column reporting.

## Best Practices

1. **Use descriptive account names:**
   ```rust
   iter.next_account("token_mint")  // Good
   iter.next_account("account_3")   // Less helpful
   ```

2. **Validate permissions early:**
   ```rust
   // Check signers and mutability at the start
   let authority = iter.next_signer("authority")?;
   let target = iter.next_mut("target")?;
   ```

3. **Use specialized methods over manual validation:**
   ```rust
   // Preferred
   let signer = iter.next_signer_mut("payer")?;

   // Avoid
   let payer = iter.next_account("payer")?;
   check_signer(payer)?;
   check_mut(payer)?;
   ```

4. **Handle optional accounts explicitly:**
   ```rust
   let optional = iter.next_option("optional_account", has_optional)?;
   ```

## Integration with Validation Functions

AccountIterator methods internally use validation functions from the `checks` module:
- `next_signer` uses `check_signer`
- `next_mut` uses `check_mut`
- `next_non_mut` uses `check_non_mut`

This ensures consistent validation across the codebase while providing enhanced error reporting.

## See Also
- [ACCOUNT_CHECKS.md](ACCOUNT_CHECKS.md) - Underlying validation functions
- [ACCOUNT_INFO_TRAIT.md](ACCOUNT_INFO_TRAIT.md) - AccountInfoTrait abstraction
- [ERRORS.md](ERRORS.md) - Complete error documentation