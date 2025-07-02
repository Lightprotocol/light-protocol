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
├── sdk-native-test/                   # Core SDK integration tests
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

- **Core SDK** (`sdk-libs/sdk/`) → `sdk-tests/sdk-native-test/`
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
• **Before + Changes = After pattern** - Test exact state transitions, not arbitrary end states
• **Complete struct assertions** - Single comprehensive `assert_eq!` on complete structs, not individual fields
• **Proper test documentation** - Numbered SUCCESS/FAIL test case lists for each test function

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

## **CRITICAL: Ideal Assertion Pattern**

**❌ WRONG: Creating arbitrary expected end states**
```rust
// Anti-pattern: Creating expected state from scratch
let expected_end_state = create_expected_state(field1, field2, field3);
assert_eq!(actual_state, expected_end_state);
```

**✅ CORRECT: Before State + Expected Changes = After State**
```rust
// IDEAL: Parse actual before state, apply expected changes, compare to after state
{
    // Parse complete state before operation
    let mut expected_after_state = parse_state_before(&state_data_before);

    // Apply the expected changes to the before state
    expected_after_state.field1 = new_value;  // Only change what should change
    expected_after_state.amount -= transfer_amount;

    // Parse actual state after operation
    let actual_after_state = parse_state_after(&state_data_after);

    // Single comprehensive assertion: after = before + changes
    assert_eq!(actual_after_state, expected_after_state);
}
```

## **Real Example from Codebase**
From `/program-tests/utils/src/assert_decompressed_token_transfer.rs`:

```rust
{
    // Parse as SPL token accounts first
    let mut sender_token_before =
        spl_token_2022::state::Account::unpack(&sender_data_before[..165]).unwrap();
    sender_token_before.amount -= transfer_amount;
    let mut recipient_token_before =
        spl_token_2022::state::Account::unpack(&recipient_data_before[..165]).unwrap();
    recipient_token_before.amount += transfer_amount;

    // Parse as SPL token accounts first
    let sender_account_after =
        spl_token_2022::state::Account::unpack(&sender_account_data.data[..165]).unwrap();
    let recipient_account_after =
        spl_token_2022::state::Account::unpack(&recipient_account_data.data[..165]).unwrap();
    assert_eq!(sender_account_after, sender_token_before);
    assert_eq!(recipient_account_after, recipient_token_before);
}
```

This pattern ensures you're testing **exact state transitions** rather than arbitrary end states.

## **Common Pitfalls and Solutions**

### **❌ Assertion Anti-Patterns to Avoid**

The test indexer in combination with litesvm LightProgram does not need time to catch up it is local.
```rust
// Give test indexer time to catch up
tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
```

```rust
// ❌ WRONG: Individual field assertions
assert_eq!(actual.field1, expected_field1);
assert_eq!(actual.field2, expected_field2);
assert_eq!(actual.field3, expected_field3);

// ❌ WRONG: Creating expected end states from scratch
let expected = ExpectedState {
    field1: "hardcoded_value",
    field2: 42,
    field3: vec![1, 2, 3],
};

// ❌ WRONG: Not capturing actual before state
let expected_before = create_expected_state(/* guess what before state was */);
```

### **✅ Correct Patterns**

```rust
// ✅ CORRECT: Parse actual before state, apply changes, assert after
let actual_before = parse_complete_state(&account_data_before);
let mut expected_after = actual_before.clone();
expected_after.field1 = new_value; // Apply only the expected change
let actual_after = parse_complete_state(&account_data_after);
assert_eq!(actual_after, expected_after);
```

### **Test Documentation Requirements**

**❌ WRONG: Vague test descriptions**
```rust
/// Test metadata operations
#[tokio::test]
async fn test_metadata() {
```

**✅ CORRECT: Numbered SUCCESS/FAIL lists**
```rust
/// Test:
/// 1. SUCCESS: Create mint with additional metadata keys
/// 2. SUCCESS: Update metadata name field
/// 3. FAIL: Update metadata field with invalid authority
#[tokio::test]
#[serial]
async fn test_metadata_field_operations() -> Result<(), RpcError> {
```

### **Error Propagation Patterns**

**❌ WRONG: Using .unwrap() everywhere**
```rust
let result = operation(&mut rpc, params).await.unwrap();
```

**✅ CORRECT: Proper error propagation**
```rust
async fn test_operation() -> Result<(), RpcError> {
    let result = operation(&mut rpc, params).await?;
    Ok(())
}
```

### **Helper Function Best Practices**

**❌ WRONG: Hiding errors in helpers**
```rust
async fn create_mint_helper(rpc: &mut RPC) {
    create_mint(rpc, params).await.unwrap(); // Hides errors!
}
```

**✅ CORRECT: Propagate errors from helpers**
```rust
async fn create_mint_helper(rpc: &mut RPC) -> Result<Signature, RpcError> {
    create_mint(rpc, params).await
}
```

### **Struct Parsing Best Practices**

**✅ CORRECT: Use borsh deserialization for easier type handling**
```rust
// Parse complete structs using borsh for easier handling
let mint_data: CompressedMint =
    BorshDeserialize::deserialize(&mut account_data.as_slice())
        .expect("Failed to deserialize CompressedMint");

// Work with the complete struct
assert_eq!(actual_mint, expected_mint);
```

**✅ CORRECT: Parse complete state, not partial data**
```rust
// Get complete account state before and after
let complete_state_before = get_complete_account_state(&mut rpc, address).await;
// ... perform operation ...
let complete_state_after = get_complete_account_state(&mut rpc, address).await;

// Apply expected changes to before state
let mut expected_after = complete_state_before.clone();
expected_after.some_field = new_value;

// Assert complete state transition
assert_eq!(complete_state_after, expected_after);
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
