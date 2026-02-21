# Light System Program

## Summary

- Core validation and coordination layer for Light Protocol
- Verifies ZK proofs for compressed account state transitions
- Manages CPI context for multi-program transactions
- Coordinates with account-compression program via CPI
- Handles SOL compression/decompression

## Used In

The Light System Program is invoked by:

- **Compressed Token Program** - All compressed token operations (mint, transfer, burn) invoke this program via CPI
- **Custom Anchor Programs** - Programs using Light SDK invoke this for compressed PDA operations via `InvokeCpi` or `InvokeCpiWithAccountInfo`
- **Direct Clients** - For simple compressed SOL transfers using the `Invoke` instruction
- **Multi-Program Transactions** - Any transaction requiring multiple programs to coordinate via shared CPI context

**Example transaction flow:**
1. Program A calls `InvokeCpiWithAccountInfo` with `first_set_context=true` to write to CPI context
2. Program B calls `InvokeCpiWithAccountInfo` with `set_context=true` to append additional data
3. Program C calls `InvokeCpiWithAccountInfo` with `execute=true` to execute the combined state transition with a single ZK proof

## Documentation

**Navigation Guide:** [docs/CLAUDE.md](docs/CLAUDE.md)

**Core Concepts:**
- [docs/PROCESSING_PIPELINE.md](docs/PROCESSING_PIPELINE.md) - 19-step processing flow (the heart of the program)
- [docs/CPI_CONTEXT.md](docs/CPI_CONTEXT.md) - Multi-program transaction coordination
- [docs/ACCOUNTS.md](docs/ACCOUNTS.md) - CpiContextAccount layouts and structures
- [docs/INSTRUCTIONS.md](docs/INSTRUCTIONS.md) - Instruction discriminators and error codes

**Instruction Details:**
- [docs/init/](docs/init/) - CPI context account initialization
- [docs/invoke/](docs/invoke/) - Direct invocation
- [docs/invoke_cpi/](docs/invoke_cpi/) - CPI invocation modes

## Key Sections

### Accounts

**CpiContextAccount (Version 2):**
- Stores instruction data across multiple CPI invocations
- Enables multi-program transactions with single ZK proof
- Default capacity: 14020 bytes (configurable via initialization parameters)
- Associated with a specific state Merkle tree

**See:** [docs/ACCOUNTS.md](docs/ACCOUNTS.md)

### Instructions

**CPI Context Management (2):**
- `InitializeCpiContextAccount` - Create new CPI context account
- `ReInitCpiContextAccount` - Migrate from version 1 to version 2

**Direct Invocation (1):**
- `Invoke` - Process compressed accounts for single program (no CPI)

**CPI Invocation (3):**
- `InvokeCpi` - Standard CPI invocation (Anchor mode)
- `InvokeCpiWithReadOnly` - CPI with read-only account support
- `InvokeCpiWithAccountInfo` - CPI with dynamic account configuration (V2 mode)

**See:** [docs/INSTRUCTIONS.md](docs/INSTRUCTIONS.md) for complete list with discriminators

### Source Code Structure

```
programs/system/src/
├── lib.rs                           # [ENTRY] Instruction dispatch, process_instruction()
├── constants.rs                     # Discriminators, program ID
├── errors.rs                        # Error definitions (6000-6066)
├── context.rs                       # Wrapped instruction data context
├── utils.rs                         # Helper functions
├── accounts/
│   ├── mod.rs                       # Account traits and exports
│   ├── init_context_account.rs     # CPI context account initialization
│   ├── account_checks.rs            # Account validation helpers
│   ├── account_traits.rs            # Traits for account access
│   ├── mode.rs                      # AccountMode (Anchor/V2)
│   └── remaining_account_checks.rs  # Remaining account validation
├── invoke/
│   ├── mod.rs
│   ├── instruction.rs               # InvokeInstruction accounts
│   └── verify_signer.rs             # Authority signature check
├── invoke_cpi/
│   ├── mod.rs
│   ├── instruction.rs               # InvokeCpiInstruction (Anchor mode)
│   ├── instruction_v2.rs            # InvokeCpiInstructionV2 (V2 mode)
│   ├── processor.rs                 # CPI invocation processing
│   └── verify_signer.rs             # CPI signer check (PDA derivation)
├── processor/
│   ├── mod.rs
│   ├── process.rs                   # [CORE] Main processing pipeline (19 steps)
│   ├── cpi.rs                       # CPI to account-compression program
│   ├── verify_proof.rs              # ZK proof verification
│   ├── sum_check.rs                 # Lamport conservation check
│   ├── sol_compression.rs           # SOL compress/decompress
│   ├── read_only_account.rs         # Read-only account verification
│   ├── read_only_address.rs         # Read-only address verification
│   ├── create_address_cpi_data.rs   # Address derivation and CPI data
│   ├── create_inputs_cpi_data.rs    # Input account processing
│   └── create_outputs_cpi_data.rs   # Output account processing
├── cpi_context/
│   ├── mod.rs
│   ├── state.rs                     # CpiContextAccount (V1 and V2)
│   ├── process_cpi_context.rs       # CPI context processing logic
│   ├── account.rs                   # CpiContextInAccount, CpiContextOutAccount
│   ├── address.rs                   # CpiContextNewAddressParamsAssignedPacked
│   └── instruction_data_trait.rs    # Trait for instruction data access
└── account_compression_state/
    ├── mod.rs
    ├── state.rs                     # State Merkle tree wrappers
    ├── address.rs                   # Address Merkle tree wrappers
    └── queue.rs                     # Queue wrappers
```

## Key Features

### 1. ZK Proof Verification
Verifies zero-knowledge proofs that validate:
- Input compressed account inclusion in state Merkle trees
- New address non-inclusion in address Merkle trees
- Read-only account inclusion
- Read-only address inclusion

### 2. CPI Context Management
Enables multiple programs to share a single ZK proof:
- First program writes instruction data to CPI context account
- Additional programs append their data
- Final program executes with combined data and one proof
- Significant savings in compute units and instruction data size

### 3. Multi-Mode Support
- **Invoke Mode:** Direct invocation for user-owned accounts
- **InvokeCpi Mode (Anchor):** CPI for program-owned accounts with Anchor-style account layout
- **V2 Mode:** CPI with dynamic account configuration - accounts passed via instruction data instead of fixed layout, reducing transaction size

### 4. SOL Compression
- Compress: Transfer SOL from user account to Sol Pool PDA, create compressed account
- Decompress: Extract compressed SOL, transfer from Sol Pool PDA to recipient

### 5. Read-Only Support
- Verify compressed accounts exist without modifying them
- Verify addresses exist without creating new ones
- Useful for authorization and multi-account validations

## Processing Pipeline

The 19-step processing flow (`src/processor/process.rs`) is the core of the program:

1. **Allocate CPI Data** - Pre-allocate memory for account-compression CPI
2. **Deserialize Accounts** - Parse and validate Merkle tree accounts
3. **Process Addresses** - Derive new addresses, verify read-only addresses
4. **Process Outputs** - Hash output accounts, validate indices
5. **Process Inputs** - Hash input accounts, create transaction hash
6. **Sum Check** - Verify lamport conservation (inputs + compress = outputs + decompress)
7. **SOL Compress/Decompress** - Transfer SOL to/from Sol Pool PDA
8. **Verify Read-Only** - Verify read-only accounts by index
9. **Verify ZK Proof** - Validate zero-knowledge proof covering all inputs/outputs/addresses
10. **Transfer Fees** - Pay network, address, and rollover fees
11. **Copy CPI Context** - Copy outputs for indexing (when using CPI context)
12. **CPI Account Compression** - Execute state transition via CPI to account-compression program

**See:** [docs/PROCESSING_PIPELINE.md](docs/PROCESSING_PIPELINE.md) for detailed step-by-step breakdown

## Error Codes

| Range | Category |
|-------|----------|
| 6000-6005 | Sum check and computation errors |
| 6006-6007 | Address errors |
| 6008-6012 | SOL compression/decompression errors |
| 6013-6019 | Validation errors |
| 6020-6028 | CPI context errors |
| 6029-6066 | Additional validation and processing errors |

**See:** [docs/INSTRUCTIONS.md](docs/INSTRUCTIONS.md) for complete list

## Testing

**Integration tests:** `program-tests/system-test/`

Tests are located in `program-tests/` because they depend on `light-test-utils` for instruction execution assertions and Solana runtime setup.

```bash
# Run all system program tests
cargo test-sbf -p system-test

# Run specific test with debugging (use long tail to see all Solana logs)
RUST_BACKTRACE=1 cargo test-sbf -p system-test -- --test test_name --nocapture 2>&1 | tail -500
```

**SDK tests:** `sdk-tests/`

```bash
# Native SDK tests
cargo test-sbf -p sdk-native-test

# Anchor SDK tests
cargo test-sbf -p sdk-anchor-test

# Token SDK tests
cargo test-sbf -p sdk-token-test
```

## Dependencies

**Program Libraries:**
- `light-account-checks` - Account validation utilities
- `light-compressed-account` - Compressed account types and instruction data
- `light-batched-merkle-tree` - Batched Merkle tree operations
- `light-hasher` - Poseidon hashing
- `light-verifier` - ZK proof verification
- `light-zero-copy` - Zero-copy serialization

**External:**
- `pinocchio` - Efficient Solana program framework
- `borsh` - Binary serialization (legacy CPI context V1)

## Program ID

```
SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7
```

## Related Programs

- **Account Compression Program** - Owns and manages Merkle tree accounts
- **Compressed Token Program** - Uses this program for all token operations
- **Registry Program** - Forester access control and protocol configuration
