# Documentation Guidelines

## 1. Crate Documentation Structure

### 1.1 Root CLAUDE.md File
Every crate must have a `CLAUDE.md` file containing:

**Required sections:**
- **Summary** - 2-5 bullet points describing crate functionality and purpose
- **Used in** - List of crates/programs that use this crate with one-liner descriptions
- **Navigation** - Instructions for navigating the documentation structure
- **High-level sections** - Major components organized by type:
  - For Solana programs: Accounts, Instructions, Source Code Structure
  - For libraries: Core Types, Public APIs, Utilities
  - For SDKs: Client Functions, Instruction Builders, Types
  - For test utilities: Test Helpers, Mock Accounts, Fixtures

**Optional sections:**
- **Config Requirements** - For programs with configuration state
- **Security Considerations** - Critical security notes

**Source Code Structure:**
Document the `src/` directory organization based on crate type:

For programs:
- **Core Instructions** - Main program operations
- **Account State** - Account structures and data layouts
- **Shared Components** - Utilities and helpers

For libraries:
- **Core Types** - Main data structures and traits

For SDKs:
- **Client Functions** - Public API methods
- **Instruction Builders** - Transaction construction
- **Types** - Shared data structures

For each module include:
- File/directory name
- Brief description of functionality
- Related features or dependencies

Example: See `programs/compressed-token/program/CLAUDE.md` Source Code Structure section

### 1.2 docs/ Directory
When documentation is extensive, create a `docs/` directory with:
- `CLAUDE.md` - Navigation guide for the docs folder
- Subdirectories for major sections (e.g., `instructions/`, `accounts/`)
- Individual `.md` files for detailed documentation

## 2. Topic-Specific Documentation

### 2.1 Account Documentation

Every account must include:

**Required fields:**
- **description** - What the account represents and its role in the program. Key concepts should be integrated here, NOT in a separate section
- **state layout** - Path to struct definition and field descriptions
- **associated instructions** - List of instructions that create/read/update/delete this account with discriminators

**For Solana accounts:**
- **discriminator** - The 8-byte discriminator value if applicable
- **size** - Account data size in bytes
- **ownership** - Expected program owner
- **serialization** - Zero-copy (programs) and Borsh (clients) examples with code snippets

**For PDAs:**
- **derivation** - Seeds used to derive the account (e.g., `[owner, program_id, mint]`)
- **bump** - Whether bump is stored or derived

**For compressed accounts:**
- **version** - Versioning scheme for data format changes
- **hashing** - Hash method (Poseidon/SHA256) and discriminator encoding
- **data layout** - Compressed data structure

**Optional fields:**
- **extensions** - Supported extension types and their layouts
- **security notes** - Critical validation requirements

**Methods/Implementations:**
For accounts with associated methods, add a Methods section with:
- Group methods by purpose (Validation, Constructors, PDA Derivation, etc.)
- Use concise parameter names in signatures
- One-line action-oriented descriptions
- Include concrete values where helpful (constants, defaults)

**Examples:**
- `programs/compressed-token/program/docs/ACCOUNTS.md`
- `program-libs/compressible/docs/CONFIG_ACCOUNT.md`

### 2.2 Instruction Documentation

Every instruction must include:

**Required sections:**
- **discriminator** - The instruction discriminator value (e.g., `18`)
- **enum** - The instruction enum variant (e.g., `CTokenInstruction::CreateTokenAccount`)
- **path** - Path to instruction processor code in the program
- **description** - High-level overview including key concepts integrated within (NOT as separate section):
  - What the instruction does
  - Key state changes
  - Usage scenarios
  - Config validation requirements (if applicable)
  - Any important notes or considerations (do NOT add a separate "Notes" section)

- **instruction_data** - Path to instruction data struct with field descriptions

- **Accounts** - Ordered list with for each account:
  - Name and type
  - Signer/writable requirements
  - Validation checks performed
  - Purpose in the instruction

- **instruction logic and checks** - Step-by-step processing:
  1. Input validation
  2. State deserialization
  3. Business logic
  4. State updates
  5. CPIs (if any)

- **Errors** - Comprehensive error list:
  - Use format: `ErrorType::Variant` (error code: N) - Description
  - Include actual numeric codes that appear in transaction logs
  - Group related errors together for clarity

**Optional sections:**
- **CPIs** - Cross-program invocations with target programs and data
- **Events** - Emitted events and their data
- **Security considerations** - Attack vectors and mitigations

**Anti-patterns to avoid:**
- Generic performance optimization comments (e.g., "uses X for performance")
- Implementation details that don't affect usage
- Internal optimizations unless they have security implications

**Examples:**
- `programs/compressed-token/program/docs/instructions/`

### 2.3 Error Documentation

Document all error codes that can be returned:

**Error format in instruction docs:**
- Use bullet list format: `ErrorType::Variant` (error code: N) - Triggering condition
- For standard Solana ProgramError variants, use their actual codes:
  - InvalidInstructionData = 3
  - InvalidAccountData = 4
  - InsufficientFunds = 6
  - MissingRequiredSignature = 8
  - NotEnoughAccountKeys = 11
  - InvalidSeeds = 14
  - (See Solana documentation for complete list)
- For custom error enums, show the u32 value that appears in transaction logs
- For errors from external crates, show them directly (e.g., `CompressibleError::InvalidState` not `ProgramError::Custom`)
- To find error codes for your program, create a test like: `programs/compressed-token/program/tests/print_error_codes.rs`

**Required for custom error documentation:**
- **Error name** - The error variant name
- **Error code** - Numeric code that appears in logs
- **Description** - What the error indicates
- **Common causes** - Typical scenarios that trigger this error
- **Resolution** - How to fix or avoid the error
- **Location** - Where error enum is defined (e.g., `anchor_compressed_token::ErrorCode`, `light_ctoken_types::CTokenError`)

**Common error crate locations in Light Protocol:**
- `anchor_compressed_token::ErrorCode` - Compressed token program errors
- `light_ctoken_types::CTokenError` - CToken type errors (18001-18037 range)
- `light_compressible::CompressibleError` - Compressible account errors (19001-19002 range)
- `light_account_checks::AccountError` - Account validation errors (12006-12021 range)
- `light_hasher::HasherError` - Hasher operation errors
- `light_compressed_account::CompressedAccountError` - Compressed account errors

**Note:** All `light-*` crates implement automatic error conversions to `ProgramError::Custom(u32)` for both pinocchio and solana_program, allowing seamless error propagation across the codebase.

**DON'Ts:**
- **DON'T document external crate errors in detail** - For errors from other crates (e.g., HasherError from light-hasher), only note the conversion exists and reference the source crate's documentation
- **DON'T include generic "best practices" sections** - Avoid preachy or overly general advice. Focus on specific, actionable information for each error
- **DON'T document ProgramError::Custom conversions** - Show the original error type directly with its code

### 2.4 Serialization Documentation

When documenting serialization:

**Zero-copy (for programs):**
```rust
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
let (data, _) = DataType::zero_copy_at(&bytes)?;
```

**Borsh (for clients):**
```rust
use borsh::BorshDeserialize;
let data = DataType::deserialize(&mut &bytes[..])?;
```

**Note:** Always specify which method to use in which context

### 2.5 CPI Documentation

For wrapper programs and CPI patterns:

**Required elements:**
- **Target program** - Program being called
- **PDA signer** - Seeds and bump for CPI authority
- **Account mapping** - How accounts are passed through
- **Data passthrough** - Instruction data handling
- **Example code** - Complete CPI invocation

## 3. Documentation Standards
- be concise and precise

### 3.1 Path References
- Always use absolute paths from repository root
- Example: `program-libs/ctoken-types/src/state/solana_ctoken.rs`

### 3.2 Code Examples
- Include working code snippets
- Show both correct usage and common mistakes
- Add inline comments explaining key points
- DON'T include print/log statements unless essential to the demonstrated functionality
- Focus on the core logic without debugging output

### 3.3 Cross-References
- Link to related documentation
- Reference source files with specific line numbers when relevant
- Use relative links within the same crate
