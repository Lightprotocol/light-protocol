


# Repository Structure
1. Solana programs (programs/*)
2. crates used in programs (program-libs/)
3. integration tests and test utilities for programs (program-tests)
4. sdks for programs (sdk-libs/)
5. integration tests for sdks (sdk-tests/)
6. circuits used in programs, prover server, and rust prover client crate (prover/)
7. forester server (forester/)






# Testing

This repository uses a comprehensive two-tier testing strategy:

- **[Unit Testing Guide](./UNIT_TESTING.md)** - For testing individual functions in isolation using mock account infos. Tests are located in `tests/` directories within each crate.

- **[Integration Testing Guide](./INTEGRATION_TESTING.md)** - For testing complete program workflows using full SVM simulation. Tests are located in the `program-tests/` directory.

## Key Testing Requirements

All tests must follow these mandatory requirements:
- **Functional test for every usage flow**
- **Failing test for every error condition**
- **Complete output verification** with single `assert_eq!` against expected structs
- **1k iteration randomized tests** for complex functions and ZeroCopy structs

# Debugging with LightProgramTest

## Transaction Log File

The light-program-test library automatically creates detailed transaction logs in:
```
target/light_program_test.log
```

### Features

- **Always enabled**: Logs are written to file regardless of environment variables
- **Clean format**: Plain text without ANSI color codes for easy reading and processing
- **Session-based**: Each test session starts with a timestamp header, transactions append to the same file
- **Comprehensive details**: Includes transaction signatures, fees, compute usage, instruction hierarchies, Light Protocol instruction parsing, and compressed account information

### Configuration

Enhanced logging is enabled by default. To disable:
```rust
let mut config = ProgramTestConfig::default();
config.enhanced_logging.enabled = false;
```

Console output requires `RUST_BACKTRACE` environment variable and can be controlled separately from file logging.

### Log File Location

The log file is automatically placed in the cargo workspace target directory, making it consistent across different test environments and working directories.

# Program Performance
- send bump seeds
- avoid deriving addresses
- avoid vectors stack over heap use ArrayVec

# Program Security

- every input (instruction data and account infos) must be checked
- inclusion of instruction data in an input compressed account data hash counts as checked

### Account checks
- ownership is checked
- cpis should use hardcoded

### Compressed accounts
- the program id is the owner of the compressed account
- data hash must be computed in the owning program
- all data that is in an input compressed account is checked implicitly by inclusion in the data hash, the data hash is part of the compressed account hash that is in the Merkle tree or queue which we prove inclusion in by zkp or index
- input compressed account
    - is existing state
    - validity is proven by index (zkp is None) or zkp
    - no data is sent to the system program
    - data hash must be computed in the owning program
- output compressed account
    - this is new state, no validity proof
    - data hash must be computed in the owning program
    - no changes to data after data hash has been computed
- minimize use of instruction data, ie do not send data twice.
    1. example, owner pubkey
       if a compressed account has an owner pubkey field which should be a tx signer, send it as signer account info, set it in the custom program, and do not sending it as instruction data. No comparison in the program is required.
    2. example, values from accounts

-

- a compressed account the state update is atomic through the cpi to the light system program, writes to the cpi context can produce non atomic transactions if solana accounts are involved and instantly updated for compressed accounts atomicity still applies, in case that a written cpi context account is not executed the state update is never actually applied only prepared.


# Zero Copies
- the derive macros ZeroCopy and ZeroCopyMut derive zero copy deserialization methods and should be used in programs
- in client code borsh is preferable
- ZeroCopy is borsh compatible
- Z and Z*Mut structs are derived by the ZeroCopy and ZeroCopyMut macros and cannot be searched with grep or rg, search for the non prefixed struct instead the zero copy struct has the same structure with zero copy types.
