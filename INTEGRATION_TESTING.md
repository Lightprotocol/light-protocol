# Integration Testing Guide

Integration tests in this repository test complete program interactions and workflows using full SVM simulation via `LightProgramTest`.

## Test Organization

### Location
Integration tests are placed in the `program-tests/` directory:

```
program-tests/
├── account-compression-test/    # Account compression program integration tests
├── client-test/                 # Client SDK integration tests
├── compressed-token-test/       # Compressed token program integration tests
├── e2e-test/                   # End-to-end integration tests
├── registry-test/              # Registry program integration tests
├── sdk-anchor-test/            # SDK anchor integration tests
├── sdk-pinocchio-test/         # SDK pinocchio integration tests
├── sdk-test/                   # Core SDK integration tests
├── sdk-token-test/             # SDK token integration tests
├── system-cpi-test/            # System CPI integration tests
├── system-cpi-v2-test/         # System CPI v2 integration tests
├── system-test/                # System program integration tests
└── utils/                      # Shared test utilities
    ├── assert_*.rs             # Assertion helper functions
    └── test_*.rs               # Test setup and utilities
```

### Coverage

**All programs in `programs/**` have corresponding integration test programs:**

- **Account Compression Program** (`programs/account-compression/`) → `program-tests/account-compression-test/`
- **Compressed Token Program** (`programs/compressed-token/`) → `program-tests/compressed-token-test/` 
- **Registry Program** (`programs/registry/`) → `program-tests/registry-test/`
- **System Program** (`programs/system/`) → `program-tests/system-test/`

**SDK libraries also have dedicated integration tests:**

- **Core SDK** (`sdk-libs/sdk/`) → `program-tests/sdk-test/`
- **Compressed Token SDK** (`sdk-libs/compressed-token-sdk/`) → `program-tests/sdk-token-test/`
- **Client SDK** (`sdk-libs/client/`) → `program-tests/client-test/`

### Basic Test Structure
```rust
use light_program_test::{LightProgramTest, ProgramTestConfig};
use serial_test::serial;

#[tokio::test]
#[serial] // Prevents race conditions between tests
async fn test_integration_workflow() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await.unwrap();
    // Test implementation...
}
```

## Testing Requirements

All integration tests in this repository must follow these mandatory requirements:

• **Functional test for every usage flow** - Each user workflow must have a corresponding test
• **Failing test for every error condition** - Every error case must have a test that verifies the expected failure
• **Complete output verification** - Assert the entire output struct in a single `assert_eq!` against the expected struct

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

## Integration Test Patterns

### Functional Test Coverage
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

### Error Test Coverage  
**Every error condition must have a failing test**

```rust
#[tokio::test]
async fn test_invalid_authority_fails() {
    let result = operation_with_wrong_authority(&mut rpc, params).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), ExpectedError::InvalidAuthority);
}
```

### Complete Output Verification
**Assert complete output structures in single `assert_eq!` against expected structs**

```rust
#[tokio::test]
async fn test_operation_output() {
    let result = perform_operation(&mut rpc, test_params).await.unwrap();
    
    let expected = ExpectedOperationResult {
        transaction_signature: result.signature,
        modified_accounts: expected_account_changes,
        emitted_events: expected_events,
        // ... all expected outputs
    };
    
    assert_eq!(result, expected);
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

## Key Components

- **RPC Client**: `LightProgramTest` provides blockchain simulation
- **Indexer**: Access via `rpc.indexer().unwrap()` for compressed account queries
- **Account Management**: Automatic keypair generation and funding
- **Transaction Execution**: `rpc.create_and_send_transaction()`

## Key Principles

### 1. Comprehensive Coverage
- **Integration tests**: Every user workflow  
- **Error tests**: Every error condition
- **Edge cases**: Boundary conditions and invalid inputs

### 2. Clear Test Structure  
- **Arrange**: Set up test data and environment
- **Act**: Execute the operation under test
- **Assert**: Verify complete expected outcome using assertion helpers

### 3. Maintainable Assertions
- Use assertion helpers from `light-test-utils`
- Assert complete structures rather than individual fields
- Provide clear error messages for assertion failures

### 4. Test Independence
- Each test should be self-contained
- Use `#[serial]` to prevent race conditions
- Clean up state between tests when necessary

## Running Integration Tests

```bash
# Run integration tests for specific package
cargo test-sbf -p compressed-token-test --all-features
cargo test-sbf -p client-test --all-features

# Run with detailed output and backtrace
RUST_BACKTRACE=1 cargo test-sbf -p <package-name> --all-features -- --nocapture

# Run specific test by name
cargo test-sbf -p compressed-token-test --all-features test_mint_lifecycle

# Run tests with custom features
cargo test-sbf -p light-batched-merkle-tree --features test-only -- --skip test_simulate_transactions
```

**Key Commands:**
- **Always use** `cargo test-sbf -p <package-name> --all-features`
- **Never use bare commands** - always specify the package
- **Use `#[serial]`** to prevent race conditions between tests

## Debugging

### Transaction Logs
The light-program-test library automatically creates detailed logs in:
```
target/light_program_test.log
```

Features:
- Always enabled regardless of environment variables
- Clean format without ANSI codes
- Session-based with timestamps
- Comprehensive transaction details including compute usage

### Common Debug Patterns
- Add print statements to trace execution flow
- Verify account states at each step of multi-step workflows
- Check transaction signatures and results
- Use the detailed logs for post-mortem analysis

## Best Practices

1. **Use descriptive test names** that explain the scenario
2. **Fund all signers** with sufficient lamports: `rpc.airdrop_lamports(&pubkey, 10_000_000_000)`
3. **Create required accounts** before operations start
4. **Use proper derivation** for PDA addresses
5. **Test both success and failure scenarios** for each workflow
6. **Verify state consistency** across all affected accounts
7. **Include all required signers** in transaction calls
8. **Handle multi-signer scenarios** correctly
9. **Test with realistic amounts** not just trivial values
10. **Verify amount conservation** in transfer operations