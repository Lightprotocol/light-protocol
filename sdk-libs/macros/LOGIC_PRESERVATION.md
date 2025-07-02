# Logic Preservation in decompress_accounts_idempotent Refactoring

## Original Logic Flow (Before Refactoring)

1. Box parameters (proof, compressed_accounts, bumps)
2. Get PDA accounts from remaining accounts
3. Validate account counts match
4. Create CPI accounts
5. Load config and get address space
6. Pre-allocate compressed_infos vector
7. FOR EACH compressed account:
   - Box the compressed_data
   - Check bounds
   - Create bump slice
   - MATCH on account variant:
     - Build seeds refs
     - Clone and box data
     - Create LightAccount
     - Call prepare_accounts_for_decompress_idempotent
     - Extend all_compressed_infos
8. IF compressed_infos not empty:
   - Create CpiInputs
   - Invoke light system program

## New Logic Flow (After Refactoring)

1. Box parameters (proof, compressed_accounts, bumps) ✅
2. Get PDA accounts from remaining accounts ✅
3. Validate account counts match ✅
4. Call setup_cpi_and_config helper:
   - Create CPI accounts ✅
   - Load config and get address space ✅
5. Pre-allocate compressed_infos vector ✅
6. FOR EACH compressed account:
   - Box the compressed_data ✅
   - Check bounds ✅
   - Call process_single_compressed_variant helper:
     - Create bump slice ✅
     - MATCH on account variant: ✅
       - Build seeds refs ✅
       - Clone and box data ✅
       - Create LightAccount ✅
       - Call prepare_accounts_for_decompress_idempotent ✅
       - Return compressed_infos ✅
   - Extend all_compressed_infos ✅
7. Call invoke_cpi_with_compressed_accounts helper:
   - IF compressed_infos not empty: ✅
     - Create CpiInputs ✅
     - Invoke light system program ✅

## What Changed

### Structural Changes Only:

- Code split into inner functions for stack management
- Helper functions defined inside main function (not at module level)
- Added lifetime parameters to ensure borrowing is correct

### What Did NOT Change:

- ✅ Same parameter boxing
- ✅ Same validation logic and error messages
- ✅ Same iteration order
- ✅ Same match statement logic
- ✅ Same seeds construction
- ✅ Same LightAccount creation
- ✅ Same CPI invocation
- ✅ Same error handling (ErrorCode::InvalidAccountCount)
- ✅ Same msg! debug statements
- ✅ Same data transformations

## Proof of Preservation

The refactoring is purely mechanical - moving code blocks into functions without changing:

1. Order of operations
2. Data transformations
3. Control flow
4. Error conditions
5. External function calls

Every single line of logic from the original is preserved, just organized into smaller stack frames.
