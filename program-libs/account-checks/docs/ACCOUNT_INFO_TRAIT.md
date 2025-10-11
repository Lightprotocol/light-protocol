# AccountInfoTrait

**Path:** `program-libs/account-checks/src/account_info/account_info_trait.rs`

## Description

AccountInfoTrait provides a unified abstraction layer over different Solana SDK AccountInfo implementations (solana-program and pinocchio). This trait enables writing SDK-agnostic validation code that works seamlessly with both SDKs without conditional compilation in business logic.

## Trait Definition

```rust
pub trait AccountInfoTrait {
    type Pubkey: Copy + Clone + Debug + PartialEq;
    type DataRef<'a>: Deref<Target = [u8]> where Self: 'a;
    type DataRefMut<'a>: DerefMut<Target = [u8]> where Self: 'a;

    // Core account access
    fn key(&self) -> [u8; 32];
    fn pubkey(&self) -> Self::Pubkey;
    fn is_writable(&self) -> bool;
    fn is_signer(&self) -> bool;
    fn executable(&self) -> bool;
    fn lamports(&self) -> u64;
    fn data_len(&self) -> usize;

    // Data borrowing
    fn try_borrow_data(&self) -> Result<Self::DataRef<'_>, AccountError>;
    fn try_borrow_mut_data(&self) -> Result<Self::DataRefMut<'_>, AccountError>;

    // Ownership and PDAs
    fn is_owned_by(&self, program: &[u8; 32]) -> bool;
    fn find_program_address(seeds: &[&[u8]], program_id: &[u8; 32]) -> ([u8; 32], u8);
    fn create_program_address(seeds: &[&[u8]], program_id: &[u8; 32]) -> Result<[u8; 32], AccountError>;

    // Rent
    fn get_min_rent_balance(size: usize) -> Result<u64, AccountError>;

    // Utilities
    fn pubkey_from_bytes(bytes: [u8; 32]) -> Self::Pubkey;
    fn data_is_empty(&self) -> bool;
}
```

## Implementations

### Solana AccountInfo
**Path:** `program-libs/account-checks/src/account_info/solana.rs`
**Feature:** `solana`

- **Pubkey type:** `solana_pubkey::Pubkey`
- **Data references:** `std::cell::Ref` / `std::cell::RefMut`
- **PDA functions:** Uses `solana_pubkey` for derivation
- **Rent:** Accesses `solana_sysvar::rent::Rent` sysvar

### Pinocchio AccountInfo
**Path:** `program-libs/account-checks/src/account_info/pinocchio.rs`
**Feature:** `pinocchio`

- **Pubkey type:** `[u8; 32]` (raw bytes for efficiency)
- **Data references:** `pinocchio::account_info::Ref` / `RefMut`
- **PDA functions:** Native pinocchio implementations on-chain, falls back to solana_pubkey off-chain
- **Rent:** Uses pinocchio sysvar access

### Test AccountInfo
**Path:** `program-libs/account-checks/src/account_info/test_account_info.rs`
**Feature:** `test-only`

Mock implementation for unit testing with configurable behavior and no external dependencies.

## Usage Examples

### Generic Function with AccountInfoTrait
```rust
use light_account_checks::{AccountInfoTrait, AccountError};

fn validate_owner<A: AccountInfoTrait>(
    account: &A,
    expected_owner: &[u8; 32],
) -> Result<(), AccountError> {
    if !account.is_owned_by(expected_owner) {
        return Err(AccountError::AccountOwnedByWrongProgram);
    }
    Ok(())
}
```

### Working with Either SDK
```rust
// With solana-program
#[cfg(feature = "solana")]
fn process_instruction_solana(
    accounts: &[solana_account_info::AccountInfo],
) -> Result<(), ProgramError> {
    let owner_account = &accounts[0];
    validate_owner(owner_account, &program_id)?;
    // ...
}

// With pinocchio
#[cfg(feature = "pinocchio")]
fn process_instruction_pinocchio(
    accounts: &[pinocchio::account_info::AccountInfo],
) -> Result<(), ProgramError> {
    let owner_account = &accounts[0];
    validate_owner(owner_account, &program_id)?;
    // Same validation code works
}
```

### PDA Derivation
```rust
fn derive_config_pda<A: AccountInfoTrait>(
    seeds: &[&[u8]],
    program_id: &[u8; 32],
) -> ([u8; 32], u8) {
    A::find_program_address(seeds, program_id)
}
```

### Data Access Pattern
```rust
fn read_discriminator<A: AccountInfoTrait>(
    account: &A,
) -> Result<[u8; 8], AccountError> {
    let data = account.try_borrow_data()?;
    if data.len() < 8 {
        return Err(AccountError::InvalidAccountSize);
    }
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&data[..8]);
    Ok(discriminator)
}
```

## Key Differences Between SDK Implementations

| Aspect | solana-program | pinocchio |
|--------|---------|-----------|
| **Pubkey Type** | `solana_pubkey::Pubkey` struct | `[u8; 32]` raw bytes |
| **Performance** | Standard | Optimized for on-chain execution |
| **Data Borrowing** | RefCell-based | Direct memory access |
| **Off-chain Support** | Full | Limited (requires fallback) |
| **Memory Overhead** | Higher | Minimal |

## Associated Types

### Pubkey
SDK-specific public key representation. Use `key()` for raw bytes when you need compatibility across SDKs.

### DataRef / DataRefMut
Smart pointers providing safe access to account data. Both dereference to `[u8]` slices but use SDK-specific memory management.

## Error Handling

All methods that can fail return `Result<T, AccountError>`:
- `try_borrow_data` / `try_borrow_mut_data` - `BorrowAccountDataFailed` (12009)
- `create_program_address` - `InvalidSeeds` (12016)
- `get_min_rent_balance` - `FailedBorrowRentSysvar` (12014)

## Best Practices

1. **Use raw bytes for keys in generic code:**
   ```rust
   fn compare_keys<A: AccountInfoTrait>(a1: &A, a2: &A) -> bool {
       a1.key() == a2.key()  // Works across SDKs
   }
   ```

2. **Handle borrow failures gracefully:**
   ```rust
   let data = account.try_borrow_data()
       .map_err(|_| AccountError::BorrowAccountDataFailed)?;
   ```

3. **Prefer trait bounds over concrete types:**
   ```rust
   fn process<A: AccountInfoTrait>(accounts: &[A]) -> Result<(), AccountError>
   ```

## See Also
- [ACCOUNT_CHECKS.md](ACCOUNT_CHECKS.md) - Validation functions using AccountInfoTrait
- [ACCOUNT_ITERATOR.md](ACCOUNT_ITERATOR.md) - Iterator pattern for account processing
- [ERRORS.md](ERRORS.md) - Error types and codes