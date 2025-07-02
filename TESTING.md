# Light Protocol Testing Guide

This repository implements a two-tier testing strategy: unit tests for isolated function testing using mocks, and integration tests for complete program workflows using full blockchain simulation. The testing philosophy emphasizes comprehensive coverage with functional tests for every usage flow, failing tests for every error condition, and complete output structure verification through single assertion comparisons.

This document outlines the testing philosophy and structure for the Light Protocol repository.

## Test Types and Organization

### 1. Unit Tests
**Location**: `tests/` directory within each workspace crate  
**Purpose**: Test individual functions in isolation  
**Environment**: No SVM, uses mock account infos from `light-account-checks`

**Account Info Setup**: If unit tests need `AccountInfo`, import from:
- `light_account_checks::account_info::test_account_info::solana_program::TestAccount`
- `light_account_checks::account_info::test_account_info::pinocchio::get_account_info`

Add feature flags only in `dev-dependencies`:
```toml
[dev-dependencies] 
light-account-checks = { path = "...", features = ["solana"] }
# or features = ["pinocchio"] depending on backend needed

# For zero-copy data structure testing:
light-batched-merkle-tree = { path = "...", features = ["test-only"] }
rand = "0.8" # For property-based testing
```

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use light_account_checks::mock::MockAccountInfo;
    
    #[test]
    fn test_function_name() {
        let mock_account = MockAccountInfo::new(/* params */);
        let result = function_under_test(&mock_account);
        assert_eq!(result, expected_value);
    }
}
```

### 2. Integration Tests  
**Location**: `program-tests/` directory  
**Purpose**: Test program interactions and workflows  
**Environment**: Full SVM simulation via `LightProgramTest`

```rust
use light_program_test::{LightProgramTest, ProgramTestConfig};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_integration_workflow() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await.unwrap();
    // Test implementation...
}
```

## Testing Requirements

All tests in this repository must follow these mandatory requirements:

• **Functional test for every usage flow** - Each user workflow must have a corresponding test
• **Failing test for every error condition** - Every error case must have a test that verifies the expected failure  
• **Complete output verification** - Assert the entire output struct in a single `assert_eq!` against the expected struct
• **Randomized test for complex functions** - Every complex function must have a randomized test with 1k iterations
• **ZeroCopy struct testing** - Every struct that derives `ZeroCopy` and `ZeroCopyMut` must have a randomized unit test with 1k iterations

### 1. Functional Test Coverage
**Every usage flow must have a functional test**

```rust
// Example: Complete user workflow test
#[tokio::test]
async fn test_complete_token_lifecycle() {
    // 1. Create mint
    // 2. Mint tokens  
    // 3. Transfer tokens
    // 4. Compress/decompress
    // Each step verified with assertions
}
```

### 2. Error Test Coverage  
**Every error condition must have a failing test**

```rust
#[tokio::test]
async fn test_invalid_authority_fails() {
    let result = operation_with_wrong_authority(&mut rpc, params).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), ExpectedError::InvalidAuthority);
}
```

### 3. Complete Output Verification
**Assert complete output structures in single `assert_eq!` against expected structs**

```rust
#[test]
fn test_complete_output() {
    let result = function_under_test(input);
    
    let expected = ExpectedOutputStruct {
        field1: expected_value1,
        field2: expected_value2,
        field3: expected_value3,
        // ... all fields
    };
    
    assert_eq!(result, expected);
}
```

## Assertion Utilities

### Location
Assertion functions should be in `program-tests/utils/light-test-utils` crate

### Structure
```rust
// program-tests/utils/light-test-utils/src/lib.rs
pub mod assert_mint;
pub mod assert_transfer;
pub mod assert_compression;

// Example assertion function
pub async fn assert_mint_operation(
    rpc: &mut LightProgramTest,
    operation_params: &OperationParams,
    expected_output: &ExpectedOutput,
) {
    // Get actual state
    let actual = get_actual_state(rpc, operation_params).await;
    
    // Single comprehensive assertion
    assert_eq!(actual, expected_output);
}
```

## Test Patterns

### Unit Test Pattern
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use light_account_checks::account_info::test_account_info::*;

    // Helper functions for creating test accounts
    #[cfg(feature = "solana")]
    fn create_test_account_solana(
        key: solana_pubkey::Pubkey,
        owner: solana_pubkey::Pubkey,
        size: usize,
        writable: bool,
    ) -> solana_program::TestAccount {
        let mut account = solana_program::TestAccount::new(key, owner, size);
        account.writable = writable;
        account
    }

    #[cfg(feature = "pinocchio")]
    fn create_test_account_pinocchio(
        key: [u8; 32],
        owner: [u8; 32],
        size: usize,
        writable: bool,
        signer: bool,
        executable: bool,
    ) -> pinocchio::account_info::AccountInfo {
        pinocchio::get_account_info(key, owner, signer, writable, executable, vec![0u8; size])
    }

    #[test]
    fn test_function_cross_backend() {
        // Test with Solana backend - Success case
        #[cfg(feature = "solana")]
        {
            let key = solana_pubkey::Pubkey::new_unique();
            let owner = solana_pubkey::Pubkey::new_unique();
            let mut account = create_test_account_solana(key, owner, 16, true);
            let result = function_under_test(&account.get_account_info());
            assert!(result.is_ok());
        }

        // Test with Solana backend - Failure case
        #[cfg(feature = "solana")]
        {
            let key = solana_pubkey::Pubkey::new_unique();
            let owner = solana_pubkey::Pubkey::new_unique();
            let mut account = create_test_account_solana(key, owner, 16, false); // Not writable
            let result = function_under_test(&account.get_account_info());
            assert_eq!(result.unwrap_err(), AccountError::AccountNotMutable);
        }

        // Test with Pinocchio backend - Success case  
        #[cfg(feature = "pinocchio")]
        {
            let key = [1u8; 32];
            let owner = [2u8; 32];
            let account = create_test_account_pinocchio(key, owner, 16, true, false, false);
            let result = function_under_test(&account);
            assert!(result.is_ok());
        }

        // Test with Pinocchio backend - Failure case
        #[cfg(feature = "pinocchio")]
        {
            let key = [1u8; 32];
            let owner = [2u8; 32];
            let account = create_test_account_pinocchio(key, owner, 16, false, false, false); // Not writable
            let result = function_under_test(&account);
            assert_eq!(result.unwrap_err(), AccountError::AccountNotMutable);
        }
    }
}
```

### Integration Test Pattern
```rust
use light_test_utils::assert_operation_result;

#[tokio::test]
#[serial]
async fn test_functional_flow() {
    let mut rpc = setup_test_environment().await;
    
    // Execute operation
    let result = perform_operation(&mut rpc, test_params).await.unwrap();
    
    // Assert complete expected outcome
    let expected = ExpectedOperationResult {
        transaction_signature: result.signature,
        modified_accounts: expected_account_changes,
        emitted_events: expected_events,
        // ... all expected outputs
    };
    
    assert_operation_result(&mut rpc, &expected).await;
}

#[tokio::test]
#[serial] 
async fn test_operation_fails_with_invalid_input() {
    let mut rpc = setup_test_environment().await;
    let invalid_params = create_invalid_test_params();
    
    let result = perform_operation(&mut rpc, invalid_params).await;
    
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "Expected error message for invalid input"
    );
}
```

## Key Principles

### 1. Comprehensive Coverage
- **Unit tests**: Every public function
- **Integration tests**: Every user workflow  
- **Error tests**: Every error condition
- **Edge cases**: Boundary conditions and invalid inputs

### 2. Clear Test Structure  
- **Arrange**: Set up test data and mocks
- **Act**: Execute the function/operation under test
- **Assert**: Verify complete expected outcome

### 3. Maintainable Assertions
- Use assertion helpers from `light-test-utils`
- Assert complete structures rather than individual fields
- Provide clear error messages for assertion failures

### 4. Test Independence
- Each test should be self-contained
- Use `#[serial]` for integration tests to prevent race conditions
- Clean up state between tests when necessary

## Example Test Organization

```
workspace-crate/
├── src/
│   ├── lib.rs
│   ├── processor.rs
│   └── instructions/
├── tests/                    # Unit tests
│   ├── processor_tests.rs
│   └── instruction_tests.rs
└── Cargo.toml

program-tests/
├── compressed-token-test/    # Integration tests
│   ├── tests/
│   │   ├── mint.rs
│   │   ├── transfer.rs  
│   │   └── compression.rs
│   └── Cargo.toml
└── utils/
    └── light-test-utils/     # Shared assertion utilities
        ├── src/
        │   ├── lib.rs
        │   ├── assert_mint.rs
        │   └── assert_transfer.rs
        └── Cargo.toml
```

## Running Tests

```bash
# Run unit tests for specific crate (always specify package with --all-features)
cargo test -p light-account-checks --all-features
cargo test -p light-zero-copy --all-features
cargo test -p light-batched-merkle-tree --all-features

# Run integration tests for specific package
cargo test-sbf -p compressed-token-test --all-features
cargo test-sbf -p client-test --all-features

# Run tests with detailed output and backtrace
RUST_BACKTRACE=1 cargo test -p <crate-name> --all-features -- --nocapture

# Run specific test by name
cargo test -p light-zero-copy --all-features test_comprehensive_api

# Skip specific tests (rare exceptions may need specific features)
cargo test -p light-batched-merkle-tree --features test-only -- --skip test_simulate_transactions --skip test_e2e
```

**Key Commands:**
- **Unit tests**: Always use `cargo test -p <crate-name> --all-features` 
- **Integration tests**: Use `cargo test-sbf -p <package-name> --all-features`
- **Never use bare `cargo test`** - always specify the package to avoid running unintended tests
- **Feature flags**: Always use `--all-features` unless specific testing scenario requires otherwise

## Unit Test Patterns from Repository

### Account Validation Tests (`/program-libs/account-checks/tests/`)
For testing functions that work with AccountInfo:

### Cross-Backend Testing
Test functions against both Solana and Pinocchio backends using feature flags:

```rust
#[test]
fn test_check_account_info() {
    // Solana success case
    #[cfg(feature = "solana")]
    {
        let owner = solana_pubkey::Pubkey::new_unique();
        let mut account = create_test_account_solana(key, owner, 16, true);
        set_discriminator::<TestStruct>(&mut account.data).unwrap();
        assert!(check_account_info::<TestStruct, _>(&owner.to_bytes(), &account.get_account_info()).is_ok());
    }

    // Pinocchio success case
    #[cfg(feature = "pinocchio")]
    {
        let owner = [2u8; 32];
        let account = create_test_account_pinocchio(key, owner, 16, true, false, false);
        account_info_init::<TestStruct, _>(&account).unwrap();
        assert!(check_account_info::<TestStruct, _>(&owner, &account).is_ok());
    }
}
```

### Comprehensive Test Documentation
Follow the account-checks pattern of documenting all test scenarios at the top:

```rust
/// Tests for all functions in module.rs:
/// 1. function_name - 4 tests
///    - Solana: Success + Failure (specific error case)
///    - Pinocchio: Success + Failure (specific error case)
/// 2. next_function - 2 tests
///    - Success + Failure (specific error)
```

### Key Patterns
- **Multiple scenarios per test**: Group success/failure cases in single test functions
- **Exact error verification**: Use `assert_eq!(result.unwrap_err(), SpecificError)` not `assert!(result.is_err())`
- **Resource management**: Properly scope borrows with `{ }` blocks when testing account data

### Data Structure Tests (`/program-libs/batched-merkle-tree/tests/`)
For testing zero-copy data structures and memory layouts:

```rust
#![cfg(feature = "test-only")] // Feature gate for test-only code

#[test]
fn test_account_init() {
    let account_size = get_account_size(params);
    let mut account_data = vec![0; account_size];
    init_function(&mut account_data, params).unwrap();
    let expected_account = create_reference_account(params);
    assert_data_structure_initialized(&mut account_data, expected_account);
}
```

### API Contract Tests (`/program-libs/zero-copy/tests/`)
For comprehensive zero-copy data structure testing:

```rust
#[test]
fn test_comprehensive_api() {
    // Test across capacity ranges
    for capacity in 1..1024 {
        let mut data = vec![0; ZeroCopyVec::required_size_for_capacity(capacity)];
        let mut vec = ZeroCopyVec::new(capacity, &mut data).unwrap();
        
        // Test all state transitions: empty -> filled -> full -> cleared
        test_empty_state(&vec, capacity);
        test_filling_state(&mut vec, capacity);
        test_full_state(&mut vec, capacity);  
        test_cleared_state(&mut vec, capacity);
    }
}

fn test_empty_state<T>(vec: &ZeroCopyVec<T>, capacity: usize) {
    // Test ALL API methods for empty state
    assert_eq!(vec.capacity(), capacity);
    assert_eq!(vec.len(), 0);
    assert!(vec.is_empty());
    assert_eq!(vec.get(0), None);
    assert_eq!(vec.first(), None);
    assert_eq!(vec.last(), None);
    assert_eq!(vec.as_slice(), &[]);
    assert!(vec.iter().next().is_none());
    // ... test every single API method
}

// Generic testing across types
#[test] 
fn test_all_type_combinations() {
    test_vec_with_types::<u8, u32>();
    test_vec_with_types::<u16, CustomStruct>();  
    test_vec_with_types::<u64, [u8; 32]>();
}

// Custom test structs with all required traits
#[derive(Copy, Clone, PartialEq, Debug, Default, 
         Immutable, FromBytes, KnownLayout, IntoBytes)]
struct TestStruct { /* fields */ }

impl Distribution<TestStruct> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> TestStruct {
        TestStruct { /* random fields */ }
    }
}
```

**Key Patterns**:
- **Exhaustive API testing**: Test every public method in every state (empty/filling/full/cleared)
- **Capacity range testing**: Test across wide range of capacity values (`1..1024`)  
- **State transition verification**: Test complete lifecycle with invariant checks
- **Memory layout validation**: Verify raw byte layout including padding and metadata
- **Generic type testing**: Test same logic across multiple type combinations
- **Boundary condition testing**: Test edge cases, error conditions, and overflow scenarios
- **Custom test data structures**: Create structs implementing all required traits for comprehensive testing
- **Helper assertion functions**: Create reusable functions that verify complete object state
- **Property-based testing**: Use `rand` with seeded RNG for 1k+ test iterations
- **Memory layout verification**: Manually calculate expected sizes and verify against actual
- **Panic testing**: Use `#[should_panic]` with specific expected messages
- **Serialization round-trips**: Serialize with Borsh, deserialize with zero-copy, compare results

## Best Practices

1. **Use descriptive test names** that explain the scenario
2. **Create test accounts** using appropriate test utilities based on test type
3. **Test multiple parameter sets** - use `test_default()`, `e2e_test_default()`, custom params
4. **Test both backends** with `#[cfg(feature = "...")]` when applicable  
5. **Use property-based testing** for complex data structures with randomized parameters
6. **Assert complete structures** or exact error types, not just success/failure
7. **Verify memory layouts** - manually calculate expected sizes for zero-copy structures
8. **Document test scenarios** at the top of test files:
   ```rust
   /// Tests for all functions in checks.rs:
   /// 1. account_info_init - 4 tests
   ///    - Solana: Success + Failure (already initialized)  
   ///    - Pinocchio: Success + Failure (already initialized)
   ```