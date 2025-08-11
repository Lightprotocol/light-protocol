# Unit Testing Guide

Unit tests in this repository test individual functions in isolation using mock account infos from `light-account-checks`. No SVM is involved.

## Test Organization

### Location
Unit tests are placed in the `tests/` directory within each workspace crate:

```
workspace-crate/
├── src/
│   ├── lib.rs
│   ├── processor.rs
│   └── instructions/
├── tests/                    # Unit tests here
│   ├── processor_tests.rs
│   └── instruction_tests.rs
└── Cargo.toml
```

### Dependencies Setup

Add feature flags only in `dev-dependencies`:
```toml
[dev-dependencies] 
light-account-checks = { path = "...", features = ["solana"] }
# or features = ["pinocchio"] depending on backend needed

# For zero-copy data structure testing:
light-batched-merkle-tree = { path = "...", features = ["test-only"] }
rand = "0.8" # For property-based testing
```

**Account Info Setup**: If unit tests need `AccountInfo`, import from:
- `light_account_checks::account_info::test_account_info::solana_program::TestAccount`
- `light_account_checks::account_info::test_account_info::pinocchio::get_account_info`

## Testing Requirements

All unit tests in this repository must follow these mandatory requirements:

• **Functional test for every usage flow** - Each user workflow must have a corresponding test
```rust
#[test]
fn test_account_info_init_success() {
    let owner = [1u8; 32];
    let account = create_test_account_pinocchio(key, owner, 16, true, false, false);
    let result = account_info_init::<TestStruct, _>(&account);
    assert!(result.is_ok());
}
```
*Example: `/program-libs/account-checks/tests/tests.rs:100` - Tests successful account initialization workflow*

• **Failing test for every error condition** - Every error case must have a test that verifies the expected failure  
```rust
#[test]
fn test_account_info_init_already_initialized() {
    let owner = [1u8; 32];
    let account = create_test_account_pinocchio(key, owner, 16, true, false, false);
    account_info_init::<TestStruct, _>(&account).unwrap(); // Initialize first time
    let result = account_info_init::<TestStruct, _>(&account); // Try again
    assert_eq!(result.unwrap_err(), AccountError::AccountAlreadyInitialized);
}
```
*Example: `/program-libs/account-checks/tests/tests.rs:120` - Tests failure when account is already initialized*

• **Complete output verification** - Assert the entire output struct in a single `assert_eq!` against the expected struct
```rust
#[test]
fn test_complete_struct_verification() {
    let result = create_test_struct(params);
    
    let expected = ExpectedStruct {
        field1: expected_value1,
        field2: expected_value2,
        field3: expected_value3,
        // ... all fields explicitly defined
    };
    
    assert_eq!(result, expected);
}
```
*Example: `/program-libs/compressed-account/src/instruction_data/zero_copy.rs:1000` - Complete CPI instruction data comparison*

• **Randomized test for complex functions** - Every complex function must have a randomized test with 1k iterations
```rust
#[test]
fn test_function_with_random_params() {
    let mut rng = StdRng::seed_from_u64(0);
    
    for _ in 0..1000 {
        let params = create_random_params(&mut rng);
        let result = complex_function(params);
        assert!(result.is_ok());
        verify_function_invariants(&result.unwrap(), &params);
    }
}
```
*Example: `/program-libs/batched-merkle-tree/tests/initialize_state_tree.rs:131` - Randomized state tree initialization test*

• **ZeroCopy struct testing** - Every struct that derives `ZeroCopy` and `ZeroCopyMut` must have a randomized unit test with 1k iterations
```rust
#[test]
fn test_zero_copy_struct_randomized() {
    let mut rng = StdRng::seed_from_u64(0);
    
    for _ in 0..1000 {
        let test_data = create_random_struct_data(&mut rng);
        let mut bytes = Vec::new();
        test_data.serialize(&mut bytes).unwrap();
        
        let (z_copy, remaining) = ZStructName::zero_copy_at(&bytes).unwrap();
        assert!(remaining.is_empty());
        
        compare_structures(&test_data, &z_copy).unwrap();
    }
}
```
*Example: `/program-libs/zero-copy/tests/vec_tests.rs:102` - Comprehensive randomized testing of ZeroCopyVec*

## Unit Test Patterns

### Account Validation Tests (`/program-libs/account-checks/tests/`)
For testing functions that work with AccountInfo:

#### Cross-Backend Testing
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

#### Comprehensive Test Documentation
Follow the account-checks pattern of documenting all test scenarios at the top:

```rust
/// Tests for all functions in checks.rs:
/// 1. account_info_init - 4 tests
///    - Solana: Success + Failure (already initialized)  
///    - Pinocchio: Success + Failure (already initialized)
/// 2. check_signer - 3 tests
///    - Solana: Failure (TestAccount always returns false)
///    - Pinocchio: Success + Failure
```

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

### Program Logic Tests (`/programs/compressed-token/program/tests/`)
For testing program-specific logic and complex business rules:

```rust
#[test]
fn test_combinatorial_scenarios() {
    // Test all combinations systematically
    let scenarios = [
        (&[100, 50], &[150], None, CompressionMode::Decompress),
        (&[75, 25, 25], &[25, 25, 25, 25, 12, 13], None, CompressionMode::Decompress),
        (&[100], &[123], Some(23), CompressionMode::Compress),
    ];
    
    for (inputs, outputs, compression, mode) in scenarios {
        let result = multi_sum_check_test(inputs, outputs, compression, mode);
        assert!(result.is_ok(), "Failed scenario: {:?}", (inputs, outputs, compression, mode));
    }
    
    // Test failure cases systematically
    let failing_scenarios = [
        (&[100, 50], &[151], None, CompressionMode::Decompress), // Wrong sum
        (&[], &[100, 50], None, CompressionMode::Decompress),    // Empty inputs
    ];
    
    for (inputs, outputs, compression, mode) in failing_scenarios {
        let result = multi_sum_check_test(inputs, outputs, compression, mode);
        assert!(result.is_err(), "Should fail: {:?}", (inputs, outputs, compression, mode));
    }
}

#[test] 
fn test_randomized_with_deterministic_seed() {
    for scenario in 0..3000 {
        let seed = scenario as u64;
        test_randomized_scenario(seed).unwrap();
    }
}

fn test_randomized_scenario(seed: u64) -> Result<()> {
    let mut rng_state = seed;
    let mut next_rand = || {
        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        rng_state
    };
    
    // Generate test parameters using deterministic randomness
    let num_inputs = 1 + (next_rand() % 6) as usize;
    let mut inputs = Vec::new();
    
    for _ in 0..num_inputs {
        let amount = 100 + (next_rand() % 1000);
        inputs.push(create_test_input(amount));
    }
    
    // Test mathematical invariants
    verify_conservation_laws(&inputs)?;
    Ok(())
}

// Helper functions for complex mock data creation
fn create_expected_input_account(/* many parameters */) -> InAccount {
    let expected_data = create_complex_structure(params);
    let expected_hash = expected_data.hash().unwrap();
    
    InAccount {
        discriminator: EXPECTED_DISCRIMINATOR,
        data_hash: expected_hash,
        merkle_context: create_merkle_context(params),
        // ... all fields explicitly set
    }
}
```
*Example: `/programs/compressed-token/program/tests/multi_sum_check.rs:14` - Comprehensive sum check testing*

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

## Key Testing Patterns

- **Multiple scenarios per test**: Group success/failure cases in single test functions
- **Exact error verification**: Use `assert_eq!(result.unwrap_err(), SpecificError)` not `assert!(result.is_err())`
- **Resource management**: Properly scope borrows with `{ }` blocks when testing account data
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

## Running Unit Tests

```bash
# Run unit tests for specific crate (always specify package with --all-features)
cargo test -p light-account-checks --all-features
cargo test -p light-zero-copy --all-features
cargo test -p light-batched-merkle-tree --all-features

# Run tests with detailed output and backtrace
RUST_BACKTRACE=1 cargo test -p <crate-name> --all-features -- --nocapture

# Run specific test by name
cargo test -p light-zero-copy --all-features test_comprehensive_api

# Skip specific tests (rare exceptions may need specific features)
cargo test -p light-batched-merkle-tree --features test-only -- --skip test_simulate_transactions --skip test_e2e
```

**Key Commands:**
- **Always use** `cargo test -p <crate-name> --all-features` 
- **Never use bare `cargo test`** - always specify the package to avoid running unintended tests
- **Feature flags**: Always use `--all-features` unless specific testing scenario requires otherwise

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