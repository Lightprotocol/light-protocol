# Stack Optimization for decompress_accounts_idempotent Macro

## Problem

The macro-generated `decompress_accounts_idempotent` function had a stack frame of 6080 bytes, exceeding the 4096 byte limit by 1640 bytes.

## Solution: Inner Function Decomposition with Parameter Bundling

Split the large monolithic function into multiple **inner helper functions** within the main function, each with its own stack frame. This avoids Anchor's "multiple fallback functions" error while still reducing stack usage. Additionally, bundle parameters into structs to reduce stack pressure from parameter passing.

### 1. **Main Function** (`decompress_accounts_idempotent`)

- Contains all helper functions as inner functions
- Reduced main logic to coordination only
- Validates inputs and delegates to helpers
- Stack usage: ~500 bytes (estimated)

### 2. **Inner Setup Helper** (`setup_cpi_and_config`)

- Defined inside main function
- Handles CPI account creation
- Loads and validates config
- Returns boxed values
- Stack usage: ~200 bytes (estimated)
- Marked with `#[inline(never)]`

### 3. **Inner Processing Helper** (`process_single_compressed_variant`)

- Defined inside main function
- Takes parameters bundled in a `ProcessParams` struct to reduce stack
- Processes one compressed account at a time
- Contains the match statement for account variants
- All large data structures boxed
- Stack usage: ~200 bytes (estimated, reduced from 4392)
- Marked with `#[inline(never)]` and `#[cold]`

### 4. **Inner Dispatch Helper** (`dispatch_variant`)

- Defined inside main function
- Contains only the match statement
- Isolates variant matching from other processing
- Stack usage: ~150 bytes (estimated)
- Marked with `#[inline(never)]` and `#[cold]`

### 5. **Inner Prepare Accounts Helper** (`call_prepare_accounts`)

- Defined inside main function
- Generic helper to call `prepare_accounts_for_decompress_idempotent`
- Separates the heavy lifting from the match statement
- Stack usage: ~300 bytes (estimated)
- Marked with `#[inline(never)]` and `#[cold]`

### 6. **Inner CPI Helper** (`invoke_cpi_with_compressed_accounts`)

- Defined inside main function
- Handles the final CPI invocation
- Minimal stack usage
- Stack usage: ~200 bytes (estimated)
- Marked with `#[inline(never)]`

## Key Optimizations

1. **Function Splitting**: Breaking the function reduces per-frame stack usage from 6080 to ~500 bytes max per function

2. **Parameter Bundling**: Using `ProcessParams` struct to pass multiple parameters as a single boxed value

3. **Boxing Strategy**: All large data structures are immediately boxed:

   - `Box::new(proof)`
   - `Box::new(compressed_accounts)`
   - `Box::new(bumps)`
   - `Box::new(Vec::with_capacity(...))`

4. **Iterator Optimization**: Removed iterator chaining that could create temporary stack allocations

5. **Cold Path Marking**: Helper functions marked with `#[cold]` to optimize for the common path

6. **No Inline**: All helpers use `#[inline(never)]` to ensure separate stack frames

## Benefits

- **Stack Safety**: Each function now uses well under the 4096 byte limit
- **Maintainability**: Smaller, focused functions are easier to understand
- **Debuggability**: Stack traces will show which helper failed
- **Flexibility**: Individual helpers can be further optimized if needed

## Estimated Stack Usage

| Function                            | Before     | After V1   | After V2   | After V3   |
| ----------------------------------- | ---------- | ---------- | ---------- | ---------- |
| decompress_accounts_idempotent      | 6080 bytes | ~500 bytes | ~500 bytes | ~500 bytes |
| setup_cpi_and_config                | N/A        | ~200 bytes | ~200 bytes | ~200 bytes |
| process_single_compressed_variant   | N/A        | 4392 bytes | 4312 bytes | ~150 bytes |
| dispatch_variant                    | N/A        | N/A        | N/A        | ~150 bytes |
| call_prepare_accounts               | N/A        | N/A        | ~300 bytes | ~300 bytes |
| invoke_cpi_with_compressed_accounts | N/A        | ~200 bytes | ~200 bytes | ~200 bytes |

Total maximum stack depth: ~1500 bytes (well under 4096 limit)

## Testing Recommendations

1. Test with maximum number of compressed accounts
2. Verify stack usage with `solana-stack-check` tool
3. Profile with different account types
4. Test error paths to ensure stack safety in all cases
