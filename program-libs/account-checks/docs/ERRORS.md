# Error Types

**Path:** `program-libs/account-checks/src/error.rs`

## Description

AccountError enum provides comprehensive error types for account validation failures. All errors automatically convert to `ProgramError::Custom(u32)` for both solana-program and pinocchio SDKs.

## Error Code Ranges

- **20000-20015**: AccountError variants
- **1-11**: Standard Pinocchio ProgramError codes (when using pinocchio feature)
- **Variable**: Pass-through codes from PinocchioProgramError

## Complete Error Reference

| Error Variant | Code | Description | Common Causes | Resolution |
|---------------|------|-------------|---------------|------------|
| `InvalidDiscriminator` | 20000 | Account discriminator mismatch | Wrong account type passed, uninitialized account | Verify correct account type, ensure account initialized |
| `AccountOwnedByWrongProgram` | 20001 | Account owner doesn't match expected | Passing system account to program expecting owned account | Check account owner before passing |
| `AccountNotMutable` | 20002 | Account not marked writable | Missing `mut` in instruction accounts | Add writable flag to account |
| `BorrowAccountDataFailed` | 20003 | Can't borrow account data | Account already borrowed, concurrent access | Ensure no overlapping borrows |
| `InvalidAccountSize` | 20004 | Account size doesn't match expected | Wrong account type, partial initialization | Verify account size matches struct |
| `AccountMutable` | 20005 | Account is writable but shouldn't be | Incorrect mutability specification | Remove writable flag from account |
| `AlreadyInitialized` | 20006 | Account discriminator already set | Attempting to reinitialize account | Check if account exists before init |
| `InvalidAccountBalance` | 20007 | Account balance below rent exemption | Insufficient lamports | Fund account to rent-exempt amount |
| `FailedBorrowRentSysvar` | 20008 | Can't access rent sysvar | Sysvar not available in context | Ensure rent sysvar accessible |
| `InvalidSigner` | 20009 | Account not a signer | Missing signature | Add account as signer in transaction |
| `InvalidSeeds` | 20010 | PDA derivation failed | Wrong seeds or bump | Verify PDA seeds and bump |
| `InvalidProgramId` | 20011 | Program account key mismatch | Wrong program passed | Pass correct program account |
| `ProgramNotExecutable` | 20012 | Program account not executable | Non-program account passed as program | Ensure account is deployed program |
| `AccountNotZeroed` | 20013 | Account data not zeroed | Account has existing data | Clear account data or use existing |
| `NotEnoughAccountKeys` | 20014 | Insufficient accounts provided | Missing required accounts | Provide all required accounts |
| `InvalidAccount` | 20015 | Account validation failed | Wrong account passed | Verify account key matches expected |
| `PinocchioProgramError` | (varies) | Pinocchio-specific error | Various SDK-level errors | Check embedded error code |

## Error Conversions

### To ProgramError

Both SDKs automatically convert AccountError to their respective ProgramError types:

```rust
// Solana
#[cfg(feature = "solana")]
impl From<AccountError> for solana_program_error::ProgramError {
    fn from(e: AccountError) -> Self {
        ProgramError::Custom(e.into())  // Converts to u32
    }
}

// Pinocchio
#[cfg(feature = "pinocchio")]
impl From<AccountError> for pinocchio::program_error::ProgramError {
    fn from(e: AccountError) -> Self {
        ProgramError::Custom(e.into())  // Converts to u32
    }
}
```

### From Pinocchio ProgramError

When using pinocchio, standard ProgramError variants map to specific codes:

| Pinocchio ProgramError | Mapped Code | AccountError Variant |
|------------------------|-------------|---------------------|
| `InvalidArgument` | 1 | `PinocchioProgramError(1)` |
| `InvalidInstructionData` | 2 | `PinocchioProgramError(2)` |
| `InvalidAccountData` | 3 | `PinocchioProgramError(3)` |
| `AccountDataTooSmall` | 4 | `PinocchioProgramError(4)` |
| `InsufficientFunds` | 5 | `PinocchioProgramError(5)` |
| `IncorrectProgramId` | 6 | `PinocchioProgramError(6)` |
| `MissingRequiredSignature` | 7 | `PinocchioProgramError(7)` |
| `AccountAlreadyInitialized` | 8 | `PinocchioProgramError(8)` |
| `UninitializedAccount` | 9 | `PinocchioProgramError(9)` |
| `NotEnoughAccountKeys` | 10 | `PinocchioProgramError(10)` |
| `AccountBorrowFailed` | 11 | `PinocchioProgramError(11)` |

### BorrowError Conversions

For solana-program SDK, RefCell borrow errors automatically convert:

```rust
#[cfg(feature = "solana")]
impl From<std::cell::BorrowError> for AccountError {
    fn from(_: std::cell::BorrowError) -> Self {
        AccountError::BorrowAccountDataFailed
    }
}

impl From<std::cell::BorrowMutError> for AccountError {
    fn from(_: std::cell::BorrowMutError) -> Self {
        AccountError::BorrowAccountDataFailed
    }
}
```

## Usage in Validation Functions

Each validation function returns specific errors:

```rust
// check_owner returns AccountOwnedByWrongProgram (20001)
check_owner(owner, account)?;

// check_signer returns InvalidSigner (20009)
check_signer(account)?;

// check_discriminator returns InvalidDiscriminator (20000) or InvalidAccountSize (20004)
check_discriminator::<MyType>(data)?;

// check_pda_seeds returns InvalidSeeds (20010)
check_pda_seeds(seeds, program_id, account)?;
```

## Error Messages in Logs

Errors appear in transaction logs with their numeric codes:

```text
Program log: ERROR: Invalid Discriminator.
Program log: Custom program error: 0x4e20  // 20000 in hex

Program log: ERROR: Not enough accounts. Requested 'mint' at index 3 but only 2 accounts available.
Program log: Custom program error: 0x4e2e  // 20014 in hex
```

## Debugging Tips

1. **Check error codes in hex**: Solana logs show custom errors in hexadecimal
   - 20000 = 0x4E20
   - 20014 = 0x4E2E
   - 20015 = 0x4E2F

2. **Use AccountIterator for detailed errors**: Provides file:line:column for debugging

3. **Common error patterns**:
   - 20000 + 20004: Usually uninitialized account
   - 20001: Wrong program ownership
   - 20009: Missing signer
   - 20014: Not enough accounts in instruction

## Integration with Other Crates

AccountError is used throughout Light Protocol:
- Validation functions in `checks` module return AccountError
- AccountIterator uses AccountError for all failures
- AccountInfoTrait methods return AccountError for borrow/PDA failures

## See Also
- [ACCOUNT_CHECKS.md](ACCOUNT_CHECKS.md) - Functions that return these errors
- [ACCOUNT_ITERATOR.md](ACCOUNT_ITERATOR.md) - Enhanced error reporting
- [ACCOUNT_INFO_TRAIT.md](ACCOUNT_INFO_TRAIT.md) - Trait methods returning AccountError
