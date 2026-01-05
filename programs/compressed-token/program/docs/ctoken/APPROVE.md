## CToken Approve

**discriminator:** 4
**enum:** `InstructionType::CTokenApprove`
**path:** programs/compressed-token/program/src/ctoken/approve_revoke.rs

### SPL Instruction Format Compatibility
This instruction is compatible with the SPL Token instruction format (using `spl_token_2022::instruction::approve` with changed program ID) when **no top-up is required**.

If the CToken account has a compressible extension and requires a rent top-up, the instruction needs the **system program account** to perform the lamports transfer. Without the system program account, the top-up CPI will fail.

**Compatibility scenarios:**
- **SPL-compatible (no system program needed):** Non-compressible accounts, or compressible accounts with sufficient prepaid rent
- **NOT SPL-compatible (system program required):** Compressible accounts that need rent top-up based on current slot

**description:**
Delegates a specified amount to a delegate authority on a decompressed ctoken account (account layout `CToken` defined in program-libs/ctoken-interface/src/state/ctoken/ctoken_struct.rs). Before the approve operation, automatically tops up compressible accounts (extension layout `CompressionInfo` defined in program-libs/compressible/src/compression_info.rs) with additional lamports if needed to prevent accounts from becoming compressible during normal operations. The instruction supports a max_top_up parameter (0 = no limit) that enforces transaction failure if the calculated top-up exceeds this limit. Uses pinocchio-token-program for SPL-compatible approve semantics. Supports backwards-compatible instruction data format (8 bytes legacy vs 10 bytes with max_top_up).

**Instruction data:**
Path: programs/compressed-token/program/src/ctoken/approve_revoke.rs (lines 34-66)

- Bytes 0-7: `amount` (u64, little-endian) - Number of tokens to delegate
- Bytes 8-9 (optional): `max_top_up` (u16, little-endian) - Maximum lamports for top-up (0 = no limit, default for legacy format)

**Accounts:**
1. source
   - (mutable)
   - The source ctoken account to approve delegation on
   - May receive rent top-up if compressible

2. delegate
   - (immutable)
   - The delegate authority who will be granted spending rights
   - Does not need to sign

3. owner
   - (signer, mutable)
   - Owner of the source account
   - Must sign the transaction
   - Acts as payer for rent top-up if compressible extension present

**Instruction Logic and Checks:**

1. **Validate minimum accounts:**
   - Require source account (index 0) and owner account (index 2)
   - Return NotEnoughAccountKeys if either account is missing
   - Note: delegate (index 1) is validated by pinocchio during SPL approve

2. **Parse instruction data:**
   - If 8 bytes: legacy format, set max_top_up = 0 (no limit)
   - If 10 bytes: parse amount (first 8 bytes) and max_top_up (last 2 bytes)
   - Return InvalidInstructionData for any other length

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

4. **Process SPL approve:**
   - Pass only first 8 bytes (amount) to pinocchio-token-program
   - Call process_approve with accounts and amount data
   - Delegate is granted spending rights for the specified amount

**Errors:**

- `ProgramError::InvalidInstructionData` (error code: 3) - Instruction data is not 8 or 10 bytes
- `ProgramError::NotEnoughAccountKeys` (error code: 11) - Less than 3 accounts provided
- `CTokenError::InvalidAccountData` (error code: 18002) - Failed to deserialize CToken account
- `CTokenError::SysvarAccessError` (error code: 18020) - Failed to get Clock or Rent sysvar for top-up calculation
- `CTokenError::MaxTopUpExceeded` (error code: 18043) - Calculated top-up exceeds max_top_up parameter
- `ProgramError::MissingRequiredSignature` (error code: 8) - Owner did not sign the transaction (SPL Token error)
- Pinocchio token errors (converted to ProgramError::Custom):
  - `TokenError::OwnerMismatch` (error code: 4) - Authority doesn't match account owner
  - `TokenError::AccountFrozen` (error code: 17) - Account is frozen

## Comparison with Token-2022

### Functional Parity

CToken Approve maintains compatibility with SPL Token-2022's core approve functionality:

**Shared Features:**
- **Delegate Authorization**: Both instructions delegate spending authority to a delegate pubkey for a specified token amount
- **Owner Signature Requirement**: Transaction must be signed by the account owner (single owner only, no multisig support in CToken)
- **Account State Validation**: Both check that the source account is initialized and not frozen
- **Delegate Field Update**: Sets `source_account.delegate` and `source_account.delegated_amount` fields
- **Backwards Compatible Data Format**: CToken supports 8-byte instruction data (amount only) for legacy compatibility

**Account Layout:**
- CToken accounts use identical base fields to Token-2022 (mint, owner, amount, delegate, state, delegated_amount, close_authority)
- Both store delegate information in the same account structure fields

### CToken-Specific Features

**1. Compressible Extension Top-Up Logic**

CToken Approve includes automatic rent top-up for accounts with the Compressible extension:

```rust
// Before SPL approve operation
process_compression_top_up(
    &ctoken.base.compression,
    account,
    &mut 0,
    &mut transfer_amount,
    &mut lamports_budget,
)?;

// Transfer lamports from owner to source if needed
if transfer_amount > 0 {
    transfer_lamports_via_cpi(transfer_amount, payer, account)?;
}
```

**Purpose**: Prevents accounts from becoming compressible during normal operations by maintaining minimum rent balance.

**Reference**: See `/home/ananas/dev/light-protocol/program-libs/compressible/docs/RENT.md` for rent calculation details.

**2. max_top_up Parameter**

Extended instruction data format (10 bytes total):
- Bytes 0-7: amount (u64)
- Bytes 8-9: max_top_up (u16, 0 = no limit)

**Enforcement**:
```rust
let lamports_budget = if max_top_up == 0 {
    u64::MAX  // No limit
} else {
    (max_top_up as u64).saturating_add(1)  // Allow exact match
};

if lamports_budget != 0 && transfer_amount > lamports_budget {
    return Err(CTokenError::MaxTopUpExceeded);
}
```

**Use Case**: Allows callers to cap unexpected rent costs and fail transactions that exceed budget.

### Unsupported SPL & Token-2022 Features

**1. No Multisig Support**
**2. No CPI Guard Extension Check**

### Related Instructions

**ApproveChecked:** CToken implements CTokenApproveChecked (discriminator: 13) with full decimals validation. See `CTOKEN_APPROVE_CHECKED.md`.
