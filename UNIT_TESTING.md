# Unit Testing Guide

Unit tests in this repository test individual functions in isolation using mock account infos from `light-account-checks`. No SVM is involved.

## General Requirements
- don't create many files
- don't use the word comprehensive in variable, test function and test file names, tests always must be compreshensive
- create a functional test for every usage flow
- create a failing test for each error
- unwraps are ok in tests but not in sdks or other library code
- structs should be asserted in one assert_eq!(expected_struct, actual_struct); assert!(result.is_ok(), is insufficient

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

### Instruction Account Validation Tests (`/programs/system/tests/`)
For testing Solana program instruction account validation (`from_account_infos()` functions):

#### Systematic Account Validation Pattern
Test all account validation rules exhaustively using mock AccountInfo helpers:

```rust
// Create systematic mock AccountInfo helpers
pub fn get_fee_payer_account_info() -> AccountInfo {
    get_account_info(
        pubkey_unique(),
        Pubkey::default(),
        true,  // is_signer
        true,  // is_writable
        false, // executable
        Vec::new(),
    )
}

pub fn get_mut_account_info() -> AccountInfo {
    get_account_info(
        pubkey_unique(),
        pubkey_unique(),
        false, // is_signer
        true,  // is_writable (this will cause validation failures)
        false, // executable
        Vec::new(),
    )
}

pub fn get_non_executable_account_compression_program_account_info() -> AccountInfo {
    get_account_info(
        ACCOUNT_COMPRESSION_PROGRAM_ID,
        pubkey_unique(),
        false, // is_signer
        false, // is_writable
        false, // executable (this will cause validation failures)
        Vec::new(),
    )
}

#[test]
fn functional_from_account_infos() {
    // Test successful account info parsing
    let fee_payer = get_fee_payer_account_info();
    let authority = get_authority_account_info();
    // ... create all required accounts

    let account_info_array = [
        fee_payer.clone(),
        authority.clone(),
        // ... all accounts in correct order
    ];

    let (instruction_struct, _) =
        InstructionStruct::from_account_infos(account_info_array.as_slice()).unwrap();

    // Verify each field is correctly parsed
    assert_eq!(instruction_struct.get_fee_payer().key(), fee_payer.key());
    assert_eq!(instruction_struct.get_authority().key(), authority.key());
    // ... verify all fields
}

#[test]
fn failing_from_account_infos() {
    // Create valid account array once
    let account_info_array = [/* all valid accounts */];

    // Test each validation failure systematically

    // 1. Authority account is mutable (should be read-only)
    {
        let mut test_accounts = account_info_array.clone();
        test_accounts[1] = get_mut_account_info();
        let result = InstructionStruct::from_account_infos(test_accounts.as_slice());
        assert_eq!(result.unwrap_err(), ProgramError::from(AccountError::AccountMutable));
    }

    // 2. Program account not executable
    {
        let mut test_accounts = account_info_array.clone();
        test_accounts[5] = get_non_executable_account_compression_program_account_info();
        let result = InstructionStruct::from_account_infos(test_accounts.as_slice());
        assert_eq!(result.unwrap_err(), ProgramError::from(AccountError::ProgramNotExecutable));
    }

    // 3. Invalid program ID
    {
        let mut test_accounts = account_info_array.clone();
        test_accounts[8] = get_mut_account_info(); // Wrong program ID
        let result = InstructionStruct::from_account_infos(test_accounts.as_slice());
        assert_eq!(result.unwrap_err(), ProgramError::from(AccountError::InvalidProgramId));
    }

    // 4. Test panic scenarios using catch_unwind
    {
        let mut test_accounts = account_info_array.clone();
        test_accounts[6] = get_mut_account_info(); // Invalid address derivation
        let result = catch_unwind(|| {
            InstructionStruct::from_account_infos(test_accounts.as_slice()).unwrap();
        });
        assert!(result.is_err(), "Expected function to panic, but it did not.");
    }
}
```
*Example: `/programs/system/tests/invoke_instruction.rs:84-172` - Exhaustive account validation testing*

#### Test Documentation Pattern
Document all test scenarios at the top of test files following the system program pattern:

```rust
/// Tests for InvokeInstruction::from_account_infos():
/// Functional tests:
/// 1. functional_from_account_infos - successful parsing with all valid accounts
/// Failing tests - each validation rule tested systematically:
/// 1. Authority mutable (should be read-only) → AccountMutable
/// 2. Registered program PDA mutable → AccountMutable
/// 3. Account compression authority mutable → AccountMutable
/// 4. Account compression program invalid ID → InvalidProgramId
/// 5. Account compression program not executable → ProgramNotExecutable
/// 6. Sol pool PDA invalid address → Panic (catch_unwind)
/// 7. System program invalid ID → InvalidProgramId
```

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

### Mathematical Property Tests (`/programs/compressed-token/program/tests/`)
For testing complex mathematical invariants and business logic:

#### Mathematical Invariant Testing Pattern
Test complex mathematical properties systematically with both success and failure cases:

```rust
#[test]
fn test_multi_sum_check() {
    // SUCCEED: Test mathematical properties that should hold
    multi_sum_check_test(&[100, 50], &[150], None, CompressionMode::Decompress).unwrap();
    multi_sum_check_test(&[75, 25, 25], &[25, 25, 25, 25, 12, 13], None, CompressionMode::Decompress).unwrap();

    // FAIL: Test violations of mathematical properties
    multi_sum_check_test(&[100, 50], &[150 + 1], None, CompressionMode::Decompress).unwrap_err();
    multi_sum_check_test(&[100, 50], &[150 - 1], None, CompressionMode::Decompress).unwrap_err();
    multi_sum_check_test(&[], &[100, 50], None, CompressionMode::Decompress).unwrap_err();

    // SUCCEED: Edge cases
    multi_sum_check_test(&[], &[], None, CompressionMode::Compress).unwrap();
    multi_sum_check_test(&[], &[], None, CompressionMode::Decompress).unwrap();

    // FAIL: Edge case violations
    multi_sum_check_test(&[], &[], Some(1), CompressionMode::Decompress).unwrap_err();
}

fn multi_sum_check_test(
    input_amounts: &[u64],
    output_amounts: &[u64],
    compress_or_decompress_amount: Option<u64>,
    compression_mode: CompressionMode,
) -> Result<()> {
    // Create test structures, serialize with Borsh
    let inputs: Vec<_> = input_amounts.iter()
        .map(|&amount| MultiInputTokenDataWithContext { amount, ..Default::default() })
        .collect();
    let input_bytes = inputs.try_to_vec().unwrap();

    // Deserialize as zero-copy and test function
    let (inputs_zc, _) = Vec::<MultiInputTokenDataWithContext>::zero_copy_at(&input_bytes).unwrap();
    sum_check_multi_mint(&inputs_zc, &outputs_zc, compressions_zc.as_deref())
}
```
*Example: `/programs/compressed-token/program/tests/multi_sum_check.rs:14` - Mathematical invariant testing*

#### Deterministic Randomized Testing Pattern
Use custom LCG for reproducible randomized testing of complex scenarios:

```rust
#[test]
fn test_multi_mint_randomized() {
    for scenario in 0..3000 {
        println!("Testing scenario {}", scenario);
        let seed = scenario as u64;
        test_randomized_scenario(seed).unwrap();
    }
}

fn test_randomized_scenario(seed: u64) -> Result<()> {
    let mut rng_state = seed;

    // Simple LCG for deterministic randomness
    let mut next_rand = || {
        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        rng_state
    };

    // Generate complex test parameters
    let num_mints = 2 + (next_rand() % 3) as usize;
    let mut mint_balances: HashMap<u8, i128> = HashMap::new();

    // Generate inputs with balance tracking
    for _ in 0..(1 + next_rand() % 6) {
        let mint = (next_rand() % num_mints as u64) as u8;
        let amount = 100 + (next_rand() % 1000);
        inputs.push((mint, amount));
        *mint_balances.entry(mint).or_insert(0) += amount as i128;
    }

    // Test mathematical invariants across all mints
    test_multi_mint_scenario(&inputs, &outputs, &compressions)
}
```
*Example: `/programs/compressed-token/program/tests/multi_sum_check.rs:150` - Deterministic randomized testing*

### Memory Layout and Allocation Tests (`/programs/compressed-token/program/tests/`)
For testing exact byte-level memory allocation and zero-copy struct layouts:

#### Exact Allocation Testing Pattern
Test precise memory allocation requirements and validate against expected struct sizes:

```rust
#[test]
fn test_exact_allocation_assertion() {
    println!("\n=== EXACT ALLOCATION TEST ===");

    // Configure dynamic metadata sizes
    let name_len = 10u32;
    let symbol_len = 5u32;
    let uri_len = 20u32;
    let additional_metadata_configs = vec![
        AdditionalMetadataConfig { key: 8, value: 15 },
        AdditionalMetadataConfig { key: 12, value: 25 },
    ];

    // Calculate expected struct size
    let mint_config = CompressedMintConfig {
        mint_authority: (true, ()),
        freeze_authority: (false, ()),
        extensions: (true, extensions_config.clone()),
    };
    let expected_mint_size = CompressedMint::byte_len(&mint_config);

    // Test allocation system
    let config = cpi_bytes_config(config_input);
    let mut cpi_bytes = allocate_invoke_with_read_only_cpi_bytes(&config);
    let (cpi_instruction_struct, _) =
        InstructionDataInvokeCpiWithReadOnly::new_zero_copy(&mut cpi_bytes[8..], config)
        .expect("Should create CPI instruction successfully");

    // Get allocated space and verify exact match
    let available_space = cpi_instruction_struct.output_compressed_accounts[0]
        .compressed_account.data.as_ref().unwrap().data.len();

    println!("Expected: {} bytes, Allocated: {} bytes", expected_mint_size, available_space);

    // Critical assertion: exact allocation match
    assert_eq!(
        available_space, expected_mint_size,
        "Allocated bytes ({}) must exactly equal CompressedMint::byte_len() ({})",
        available_space, expected_mint_size
    );
}
```
*Example: `/programs/compressed-token/program/tests/exact_allocation_test.rs:12` - Exact allocation testing*

### Mock Data Generation Tests (`/programs/compressed-token/program/tests/`)
For testing complex program logic with realistic mock data:

#### Systematic Mock Generation Pattern
Create comprehensive mock data with systematic parameter variations:

```rust
#[test]
fn test_rnd_create_input_compressed_account() {
    let mut rng = rand::thread_rng();
    let iter = 1000;

    for _ in 0..iter {
        // Generate realistic random parameters
        let mint_pubkey = Pubkey::new_from_array(rng.gen::<[u8; 32]>());
        let owner_pubkey = Pubkey::new_from_array(rng.gen::<[u8; 32]>());
        let amount = rng.gen::<u64>();
        let with_delegate = rng.gen_bool(0.3); // 30% probability

        // Create complex input structure
        let input_token_data = MultiInputTokenDataWithContext {
            amount,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: rng.gen_range(0..=255u8),
                queue_pubkey_index: rng.gen_range(0..=255u8),
                leaf_index: rng.gen::<u32>(),
                prove_by_index: rng.gen_bool(0.5),
            },
            root_index: rng.gen::<u16>(),
            with_delegate,
            // ... complex conditional logic
        };

        // Create systematic mock accounts based on parameters
        let mut mock_accounts = vec![
            create_mock_account(mint_pubkey, false),
            create_mock_account(owner_pubkey, !with_delegate), // signer logic
        ];

        if with_delegate {
            mock_accounts.push(create_mock_account(delegate_pubkey, true));
        }

        // Test both frozen and unfrozen states systematically
        for is_frozen in [false, true] {
            test_account_setup(&input_token_data, &mock_accounts, is_frozen);
        }
    }
}

fn create_mock_account(pubkey: Pubkey, is_signer: bool) -> AccountInfo {
    get_account_info(
        pubkey,
        Pubkey::default(),
        is_signer,  // Conditional signer status
        false,      // writable
        false,      // executable
        Vec::new(),
    )
}
```
*Example: `/programs/compressed-token/program/tests/token_input.rs:28` - Systematic mock data generation*

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

### Core Patterns
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

### Solana Program-Specific Patterns
- **Systematic account validation testing**: Create one functional test and one comprehensive failing test for each `from_account_infos()` function
- **Mock AccountInfo helpers**: Create systematic helper functions for different account scenarios (mutable, non-executable, wrong program ID, etc.)
- **Exhaustive error case coverage**: Test every validation rule failure mode individually with specific error assertions
- **Panic scenario testing**: Use `std::panic::catch_unwind()` for testing functions expected to panic during invalid account validation
- **Cross-program account validation**: Test accounts that must validate across multiple programs (system, compression, token programs)
- **Account property isolation**: Test each account property (mutability, executability, program ownership, signer status) independently
- **Account array manipulation**: Clone base valid array and modify individual positions to test specific failure scenarios
- **Test documentation headers**: Document all test scenarios systematically at the top of test files

### Advanced Testing Patterns (From Compressed-Token Program)
- **Mathematical invariant testing**: Test complex business rules and mathematical properties systematically with both success/failure cases
- **Deterministic randomization**: Use custom LCG (Linear Congruential Generator) for reproducible random testing scenarios by seed
- **Balance tracking in randomized tests**: Maintain complex state (HashMap) during multi-entity randomized testing to verify conservation laws
- **Exact memory allocation testing**: Test precise byte-level allocation requirements and validate against expected struct sizes with detailed logging
- **Systematic mock data generation**: Create realistic mock data with probability-based parameters and conditional account relationships
- **Serialization pipeline testing**: Explicitly test borsh → zero-copy conversion pipeline with round-trip verification
- **Multi-parameter combinatorial testing**: Test all combinations of parameters systematically (modes, amounts, account states)
- **Dynamic sizing validation**: Test variable-length data structures and verify padding/alignment overhead
- **State-dependent mock generation**: Create mock data that adapts based on generated parameters (conditional delegate accounts, balance-aware compressions)
- **Complex scenario debugging**: Use detailed println! logging and deterministic seeds for reproducible debugging of failing scenarios

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

### General Testing
1. **Use descriptive test names** that explain the scenario
2. **Create test accounts** using appropriate test utilities based on test type
3. **Test multiple parameter sets** - use `test_default()`, `e2e_test_default()`, custom params
4. **Test both backends** with `#[cfg(feature = "...")]` when applicable
5. **Use property-based testing** for complex data structures with randomized parameters
6. **Assert complete structures** or exact error types, not just success/failure
7. **Verify memory layouts** - manually calculate expected sizes for zero-copy structures

### Solana Program Testing
8. **Follow the 2-test pattern**: One `functional_*` test (success case) + one `failing_*` test (all error cases) per `from_account_infos()` function
9. **Create systematic mock helpers**: Build a comprehensive set of `get_*_account_info()` functions covering all account variations needed for testing
10. **Test every validation rule**: Each account validation check must have a corresponding failing test case with exact error assertion
11. **Use block scoping**: Isolate each failing test case in `{ }` blocks with descriptive comments
12. **Import `std::panic::catch_unwind`** for testing functions that panic on invalid account derivations
13. **Document test scenarios** systematically at the top of test files:
   ```rust
   /// Tests for InvokeInstruction::from_account_infos():
   /// Functional tests:
   /// 1. functional_from_account_infos - successful parsing with all valid accounts
   /// Failing tests - each validation rule tested systematically:
   /// 1. Authority mutable (should be read-only) → AccountMutable
   /// 2. Registered program PDA mutable → AccountMutable
   /// 3. Account compression authority mutable → AccountMutable
   /// 4. Account compression program invalid ID → InvalidProgramId
   /// 5. Account compression program not executable → ProgramNotExecutable
   /// 6. Sol pool PDA invalid address → Panic (catch_unwind)
   /// 7. System program invalid ID → InvalidProgramId
   ```

### Advanced Testing (Complex Business Logic)
14. **Test mathematical invariants**: For functions implementing complex business rules, create systematic success/failure test cases that verify mathematical properties (balance conservation, sum checks, etc.)
15. **Use deterministic randomization**: Implement custom LCG (`rng_state.wrapping_mul(1103515245).wrapping_add(12345)`) for reproducible randomized tests where specific failing scenarios can be debugged by seed number
16. **Track complex state**: When testing multi-entity operations, use HashMap or similar to track state changes and verify invariants across all entities
17. **Test exact memory allocation**: For zero-copy structs with dynamic sizing, calculate expected byte sizes manually and assert exact allocation matches with detailed logging
18. **Create realistic mock data**: Use probability-based parameter generation (`rng.gen_bool(0.3)`) and conditional account relationships that mirror real usage patterns
19. **Test serialization pipelines**: Explicitly test the borsh serialization → zero-copy deserialization → function call pipeline to ensure data integrity
20. **Use detailed debugging output**: Include comprehensive `println!` logging in complex randomized tests to enable debugging of failing scenarios
21. **Test parameter combinations**: For functions with multiple modes/parameters, systematically test all valid combinations and edge cases
