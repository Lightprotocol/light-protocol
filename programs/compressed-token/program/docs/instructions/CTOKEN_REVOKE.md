## CToken Revoke

**discriminator:** 5
**enum:** `InstructionType::CTokenRevoke`
**path:** programs/compressed-token/program/src/ctoken_approve_revoke.rs

### SPL Instruction Format Compatibility

**Important:** This instruction is only compatible with the SPL Token instruction format (using `spl_token_2022::instruction::revoke` with changed program ID) when **no top-up is required**.

If the CToken account has a compressible extension and requires a rent top-up, the instruction needs the **system program account** to perform the lamports transfer. Without the system program account, the top-up CPI will fail.

**Compatibility scenarios:**
- **SPL-compatible (no system program needed):** Non-compressible accounts, or compressible accounts with sufficient prepaid rent
- **NOT SPL-compatible (system program required):** Compressible accounts that need rent top-up based on current slot

**description:**
Revokes any previously granted delegation on a decompressed ctoken account (account layout `CToken` defined in program-libs/ctoken-interface/src/state/ctoken/ctoken_struct.rs). Before the revoke operation, automatically tops up compressible accounts (extension layout `CompressionInfo` defined in program-libs/compressible/src/compression_info.rs) with additional lamports if needed to prevent accounts from becoming compressible during normal operations. The instruction supports a max_top_up parameter (0 = no limit) that enforces transaction failure if the calculated top-up exceeds this limit. Uses pinocchio-token-program for SPL-compatible revoke semantics. Supports backwards-compatible instruction data format (0 bytes legacy vs 2 bytes with max_top_up). The revoke operation follows SPL Token rules exactly (clears delegate and delegated_amount).

**Instruction data:**
Path: programs/compressed-token/program/src/ctoken_approve_revoke.rs (lines 71-106)

- Empty (0 bytes): legacy format, no max_top_up enforcement (max_top_up = 0, no limit)
- Bytes 0-1 (optional): `max_top_up` (u16, little-endian) - Maximum lamports for top-up (0 = no limit)

**Accounts:**
1. source
   - (mutable)
   - The source ctoken account to revoke delegation on
   - May receive rent top-up if compressible

2. owner
   - (signer, mutable)
   - Owner of the source account
   - Must sign the transaction
   - Acts as payer for rent top-up if compressible extension present

**Instruction Logic and Checks:**

1. **Parse instruction data:**
   - If 0 bytes: legacy format, set max_top_up = 0 (no limit)
   - If 2 bytes: parse max_top_up (u16, little-endian)
   - Return InvalidInstructionData for any other length

2. **Validate minimum accounts:**
   - Require at least 2 accounts (source, owner)
   - Return NotEnoughAccountKeys if insufficient

3. **Process compressible top-up:**
   - Borrow source account data mutably
   - Deserialize CToken using zero-copy validation
   - Initialize lamports_budget based on max_top_up:
     - If max_top_up == 0: budget = u64::MAX (no limit)
     - Otherwise: budget = max_top_up + 1 (allows exact match)
   - Call process_compression_top_up with source account's compression info
   - Drop borrow before CPI
   - If transfer_amount > 0:
     - Check that transfer_amount <= lamports_budget
     - Return MaxTopUpExceeded if budget exceeded
     - Transfer lamports from owner to source via CPI

4. **Process revoke (inline via pinocchio-token-program library):**
   - Call process_revoke with accounts
   - Clears the delegate field and delegated_amount on the source account

**Errors:**

- `ProgramError::InvalidInstructionData` (error code: 3) - Instruction data is not 0 or 2 bytes
- `ProgramError::NotEnoughAccountKeys` (error code: 11) - Less than 2 accounts provided
- `CTokenError::InvalidAccountData` (error code: 18002) - Failed to deserialize CToken account
- `CTokenError::SysvarAccessError` (error code: 18020) - Failed to get Clock or Rent sysvar for top-up calculation
- `CTokenError::MaxTopUpExceeded` (error code: 18043) - Calculated top-up exceeds max_top_up parameter
- `ProgramError::MissingRequiredSignature` (error code: 8) - Owner did not sign the transaction (SPL Token error)
- Pinocchio token errors (converted to ProgramError::Custom):
  - `TokenError::OwnerMismatch` (error code: 4) - Authority doesn't match account owner
  - `TokenError::AccountFrozen` (error code: 17) - Account is frozen

## Comparison with SPL Token

### Functional Parity

CToken delegates core logic to `pinocchio_token_program::processor::revoke::process_revoke`, which implements SPL Token-compatible revoke semantics:
- Delegate clearing, owner authority validation, frozen account check

### CToken-Specific Features

**1. Compressible Top-Up Logic**
Automatically tops up accounts with rent lamports before revoking to prevent accounts from becoming compressible.

**2. max_top_up Parameter**
2-byte instruction format adds `max_top_up` (u16) to limit top-up costs. Fails with `MaxTopUpExceeded` (18043) if exceeded.

### Unsupported SPL & Token-2022 Features

**1. No Multisig Support**
**2. No Dual Authority Model** - Token-2022 allows both owner AND delegate to revoke; CToken only accepts owner
**3. No CPI Guard Extension Check**
