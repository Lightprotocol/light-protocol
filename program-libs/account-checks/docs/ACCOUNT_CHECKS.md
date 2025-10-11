# Account Validation Functions

**Path:** `program-libs/account-checks/src/checks.rs`

## Description

Comprehensive validation functions for Solana account verification. All functions are generic over `AccountInfoTrait`, enabling use with both Solana and Pinocchio runtimes.

## Core Validation Functions

### Ownership Validation

#### `check_owner`
```rust
fn check_owner<A: AccountInfoTrait>(
    owner: &[u8; 32],
    account_info: &A
) -> Result<(), AccountError>
```
- Verifies account is owned by specified program
- **Error:** `AccountOwnedByWrongProgram` (20001)

#### `check_program`
```rust
fn check_program<A: AccountInfoTrait>(
    program_id: &[u8; 32],
    account_info: &A
) -> Result<(), AccountError>
```
- Verifies account key matches program_id AND is executable
- **Errors:**
  - `InvalidProgramId` (20011) - Key mismatch
  - `ProgramNotExecutable` (20012) - Not marked executable

### Permission Validation

#### `check_signer`
```rust
fn check_signer<A: AccountInfoTrait>(
    account_info: &A
) -> Result<(), AccountError>
```
- Verifies account is transaction signer
- **Error:** `InvalidSigner` (20009)

#### `check_mut`
```rust
fn check_mut<A: AccountInfoTrait>(
    account_info: &A
) -> Result<(), AccountError>
```
- Verifies account is writable
- **Error:** `AccountNotMutable` (20002)

#### `check_non_mut`
```rust
fn check_non_mut<A: AccountInfoTrait>(
    account_info: &A
) -> Result<(), AccountError>
```
- Verifies account is NOT writable
- **Error:** `AccountMutable` (20005)

### Discriminator Functions

#### `check_discriminator`
```rust
fn check_discriminator<T: Discriminator>(
    bytes: &[u8]
) -> Result<(), AccountError>
```
- Verifies first 8 bytes match expected discriminator
- **Errors:**
  - `InvalidAccountSize` (20004) - Less than 8 bytes
  - `InvalidDiscriminator` (20000) - Mismatch

#### `set_discriminator`
```rust
fn set_discriminator<T: Discriminator>(
    bytes: &mut [u8]
) -> Result<(), AccountError>
```
- Sets 8-byte discriminator on uninitialized account
- **Error:** `AlreadyInitialized` (20006) - Non-zero discriminator

#### `account_info_init`
```rust
fn account_info_init<T: Discriminator, A: AccountInfoTrait>(
    account_info: &A
) -> Result<(), AccountError>
```
- Initializes account with discriminator
- **Errors:**
  - `BorrowAccountDataFailed` (20003)
  - `AlreadyInitialized` (20006)

### Combined Validators

#### `check_account_info`
```rust
fn check_account_info<T: Discriminator, A: AccountInfoTrait>(
    program_id: &[u8; 32],
    account_info: &A
) -> Result<(), AccountError>
```
Validates:
1. Ownership by program_id
2. Discriminator matches type T

#### `check_account_info_mut`
```rust
fn check_account_info_mut<T: Discriminator, A: AccountInfoTrait>(
    program_id: &[u8; 32],
    account_info: &A
) -> Result<(), AccountError>
```
Validates:
1. Account is writable
2. Ownership by program_id
3. Discriminator matches type T

#### `check_account_info_non_mut`
```rust
fn check_account_info_non_mut<T: Discriminator, A: AccountInfoTrait>(
    program_id: &[u8; 32],
    account_info: &A
) -> Result<(), AccountError>
```
Validates:
1. Account is NOT writable
2. Ownership by program_id
3. Discriminator matches type T

### PDA Validation

#### `check_pda_seeds`
```rust
fn check_pda_seeds<A: AccountInfoTrait>(
    seeds: &[&[u8]],
    program_id: &[u8; 32],
    account_info: &A
) -> Result<(), AccountError>
```
- Derives PDA and verifies it matches account key
- Uses `find_program_address` (finds bump)
- **Error:** `InvalidSeeds` (20010)

#### `check_pda_seeds_with_bump`
```rust
fn check_pda_seeds_with_bump<A: AccountInfoTrait>(
    seeds: &[&[u8]],  // Must include bump
    program_id: &[u8; 32],
    account_info: &A
) -> Result<(), AccountError>
```
- Verifies PDA with known bump seed
- Uses `create_program_address` (requires bump)
- **Error:** `InvalidSeeds` (20010)

### Rent Validation

#### `check_account_balance_is_rent_exempt`
```rust
fn check_account_balance_is_rent_exempt<A: AccountInfoTrait>(
    account_info: &A,
    expected_size: usize
) -> Result<u64, AccountError>
```
- Verifies account size and rent exemption
- Returns rent exemption amount
- **Errors:**
  - `InvalidAccountSize` (20004) - Size mismatch
  - `InvalidAccountBalance` (20007) - Below rent exemption
  - `FailedBorrowRentSysvar` (20008) - Can't access rent

### Initialization Check

#### `check_data_is_zeroed`
```rust
fn check_data_is_zeroed<const N: usize>(
    data: &[u8]
) -> Result<(), AccountError>
```
- Verifies first N bytes are zero (uninitialized)
- **Error:** `AccountNotZeroed` (20013)

## Usage Examples

### Initialize New Account
```rust
use light_account_checks::checks::{account_info_init, check_account_balance_is_rent_exempt};

struct MyAccount;
impl Discriminator for MyAccount {
    const LIGHT_DISCRIMINATOR: [u8; 8] = [180, 4, 231, 26, 220, 144, 55, 168];
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
}

fn initialize_account<A: AccountInfoTrait>(
    account: &A,
    expected_size: usize,
) -> Result<(), AccountError> {
    // Check rent exemption
    check_account_balance_is_rent_exempt(account, expected_size)?;

    // Set discriminator
    account_info_init::<MyAccount, A>(account)?;

    Ok(())
}
```

### Validate Mutable Account
```rust
use light_account_checks::checks::check_account_info_mut;

fn process_update<A: AccountInfoTrait>(
    program_id: &[u8; 32],
    account: &A,
) -> Result<(), AccountError> {
    // Validates: writable + owned by program + correct discriminator
    check_account_info_mut::<MyAccount, A>(program_id, account)?;

    // Safe to modify account data
    let mut data = account.try_borrow_mut_data()?;
    // ... modifications
    Ok(())
}
```

### Validate PDA
```rust
use light_account_checks::checks::{check_pda_seeds, check_owner};

fn validate_config_pda<A: AccountInfoTrait>(
    program_id: &[u8; 32],
    config_account: &A,
    authority: &[u8; 32],
) -> Result<(), AccountError> {
    // Build seeds
    let seeds = &[b"config", authority.as_ref()];

    // Verify PDA derivation
    check_pda_seeds(seeds, program_id, config_account)?;

    // Verify ownership
    check_owner(program_id, config_account)?;

    Ok(())
}
```

### Combined Validation Pattern
```rust
fn process_instruction<A: AccountInfoTrait>(
    program_id: &[u8; 32],
    accounts: &[A],
) -> Result<(), AccountError> {
    let authority = &accounts[0];
    let target = &accounts[1];
    let config = &accounts[2];

    // Authority must sign
    check_signer(authority)?;

    // Target must be mutable and owned by program
    check_account_info_mut::<TokenAccount, A>(program_id, target)?;

    // Config must be read-only
    check_account_info_non_mut::<ConfigAccount, A>(program_id, config)?;

    Ok(())
}
```

## Validation Patterns

### Account State Machine
```rust
// 1. Uninitialized -> check zeroed discriminator
check_data_is_zeroed::<8>(&account_data)?;

// 2. Initialize -> set discriminator
set_discriminator::<MyType>(&mut account_data)?;

// 3. Initialized -> validate discriminator
check_discriminator::<MyType>(&account_data)?;
```

### PDA with Stored vs Derived Bump
```rust
// If bump is stored in account data
let seeds_with_bump = &[b"config", authority.as_ref(), &[stored_bump]];
check_pda_seeds_with_bump(seeds_with_bump, program_id, account)?;

// If bump needs to be found
let seeds = &[b"config", authority.as_ref()];
check_pda_seeds(seeds, program_id, account)?;
```

## Error Reference

| Function | Error Code | Error Name | Condition |
|----------|------------|------------|-----------|
| `check_owner` | 20001 | AccountOwnedByWrongProgram | Owner mismatch |
| `check_mut` | 20002 | AccountNotMutable | Not writable |
| `check_discriminator` | 20000 | InvalidDiscriminator | Wrong type |
| `check_discriminator` | 20004 | InvalidAccountSize | < 8 bytes |
| `set_discriminator` | 20006 | AlreadyInitialized | Non-zero disc |
| `check_non_mut` | 20005 | AccountMutable | Is writable |
| `check_signer` | 20009 | InvalidSigner | Not signer |
| `check_pda_seeds*` | 20010 | InvalidSeeds | PDA mismatch |
| `check_program` | 20011 | InvalidProgramId | Key mismatch |
| `check_program` | 20012 | ProgramNotExecutable | Not executable |
| `check_data_is_zeroed` | 20013 | AccountNotZeroed | Has data |
| `check_account_balance_*` | 20004 | InvalidAccountSize | Size mismatch |
| `check_account_balance_*` | 20007 | InvalidAccountBalance | Low balance |
| `check_account_balance_*` | 20008 | FailedBorrowRentSysvar | Can't get rent |
| `account_info_init` | 20003 | BorrowAccountDataFailed | Can't borrow |

## See Also
- [ACCOUNT_INFO_TRAIT.md](ACCOUNT_INFO_TRAIT.md) - AccountInfoTrait abstraction
- [DISCRIMINATOR.md](DISCRIMINATOR.md) - Discriminator trait details
- [ACCOUNT_ITERATOR.md](ACCOUNT_ITERATOR.md) - Sequential validation
- [ERRORS.md](ERRORS.md) - Complete error documentation
