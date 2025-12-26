## CToken Approve

**discriminator:** 4
**enum:** `InstructionType::CTokenApprove`
**path:** programs/compressed-token/program/src/ctoken_approve_revoke.rs

### SPL Instruction Format Compatibility

**Important:** This instruction is only compatible with the SPL Token instruction format (using `spl_token_2022::instruction::approve` with changed program ID) when **no top-up is required**.

If the CToken account has a compressible extension and requires a rent top-up, the instruction needs the **system program account** to perform the lamports transfer. Without the system program account, the top-up CPI will fail.

**Compatibility scenarios:**
- **SPL-compatible (no system program needed):** Non-compressible accounts, or compressible accounts with sufficient prepaid rent
- **NOT SPL-compatible (system program required):** Compressible accounts that need rent top-up based on current slot

**description:**
Delegates a specified amount to a delegate authority on a decompressed ctoken account (account layout `CToken` defined in program-libs/ctoken-interface/src/state/ctoken/ctoken_struct.rs). Before the approve operation, automatically tops up compressible accounts (extension layout `CompressionInfo` defined in program-libs/compressible/src/compression_info.rs) with additional lamports if needed to prevent accounts from becoming compressible during normal operations. The instruction supports a max_top_up parameter (0 = no limit) that enforces transaction failure if the calculated top-up exceeds this limit. Uses pinocchio-token-program for SPL-compatible approve semantics. Supports backwards-compatible instruction data format (8 bytes legacy vs 10 bytes with max_top_up).

**Instruction data:**
Path: programs/compressed-token/program/src/ctoken_approve_revoke.rs (lines 22-46)

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

1. **Parse instruction data:**
   - If 8 bytes: legacy format, set max_top_up = 0 (no limit)
   - If 10 bytes: parse amount (first 8 bytes) and max_top_up (last 2 bytes)
   - Return InvalidInstructionData for any other length

2. **Validate minimum accounts:**
   - Require at least 3 accounts (source, delegate, owner)
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

### Missing Features

**1. No Multisig Support**

**Token-2022 Multisig Flow:**
```
Accounts (Multisig):
0. [writable] Source account
1. [] Delegate
2. [] Multisignature owner account
3. ..3+M [signer] M signer accounts
```

**CToken Limitation:**
- Only supports single owner signature
- No multisignature account validation
- Requires exactly 3 accounts (source, delegate, owner)

**Impact**: Users requiring M-of-N signature schemes cannot use CToken accounts for approval operations.

**2. No CPI Guard Extension Check**

**Token-2022 CPI Guard Protection:**
```rust
// Token-2022 processor.rs:611-615
if let Ok(cpi_guard) = source_account.get_extension::<CpiGuard>() {
    if cpi_guard.lock_cpi.into() && in_cpi() {
        return Err(TokenError::CpiGuardApproveBlocked);
    }
}
```

**CToken Behavior:**
- Does NOT check for CPI Guard extension
- Does NOT prevent approval via Cross-Program Invocation
- No extension validation beyond Compressible

**Security Implication**: CToken accounts cannot use CPI Guard to prevent opaque programs from gaining approval authority during CPIs. This is a deliberate design choice as CToken focuses on compression functionality rather than all Token-2022 extensions.

**3. No ApproveChecked Variant**

**Token-2022 ApproveChecked:**
```
Instruction Data:
- amount: u64
- decimals: u8

Additional Account:
1. [] The token mint

Additional Checks:
- Validates source_account.mint == mint_info.key
- Validates expected_decimals == mint.base.decimals
```

**CToken Status:**
- Only implements basic Approve (no mint/decimals validation)
- No ApproveChecked instruction variant
- Relies on caller to ensure correct mint context

**Risk**: Without mint validation, callers could potentially approve on wrong token accounts if not carefully validating mint addresses externally.

### Extension Handling Differences

| Extension | Token-2022 Approve | CToken Approve |
|-----------|-------------------|----------------|
| **CPI Guard** | Blocks approval via CPI when enabled | Not checked, allows approval via CPI |
| **Compressible** | N/A (Token-2022 extension, not in standard T22) | Auto top-up with max_top_up enforcement |
| **Account State** | Checks initialized and frozen state | Delegates to pinocchio (same checks) |
| **Multisig** | Validates M-of-N signatures with position matching | Not supported |

### Security Property Comparison

Based on Token-2022 security analysis (`/home/ananas/dev/token-2022/analysis/approve.md`):

**Shared Security Properties:**
1. **Account Initialization Check**: Both verify source account is initialized (via unpack validation)
2. **Account Frozen State Validation**: Both prevent approval when account is frozen
3. **Owner Authority Validation**: Both validate owner signature matches account owner field

**Token-2022 Additional Security:**
1. **Mint Validation** (ApproveChecked): Validates source account mint matches provided mint
2. **Decimals Validation** (ApproveChecked): Validates expected decimals match mint decimals
3. **CPI Guard Check**: Prevents approval via CPI when guard enabled
4. **Multisig Validation**: M-of-N signature validation with position matching

**CToken Additional Security:**
1. **Rent Budget Enforcement**: max_top_up parameter prevents unexpected rent costs
2. **Compressible State Management**: Ensures accounts maintain minimum rent to prevent compression

**Critical Security Gap (Token-2022):**
According to the security analysis, Token-2022's Approve instruction is missing explicit account ownership validation (`check_program_account(source_account_info.owner)?`). CToken delegates to pinocchio-token-program, which inherits this same gap. This is MEDIUM severity as the owner validation check provides significant protection, but the missing program ownership check creates potential attack surface if combined with account confusion attacks.

**Recommendation for CToken:**
- Current implementation correctly delegates to pinocchio for SPL compatibility
- If pinocchio addresses the account ownership validation gap, CToken will automatically inherit the fix
- Consider adding explicit ownership validation in CToken layer before delegating to pinocchio

### Summary

**Use CToken Approve when:**
- Working with compressed token accounts that may need rent top-up
- Need to enforce maximum rent cost budget (max_top_up parameter)
- Only require single owner signature
- CPI Guard protection is not required

**Use Token-2022 Approve when:**
- Need multisignature approval support
- Require CPI Guard protection against opaque CPI approvals
- Want mint/decimals validation (ApproveChecked variant)
- Working with standard Token-2022 accounts without compression

**Migration Path:**
Users can decompress CToken accounts to Token-2022 accounts to gain access to multisig and CPI Guard features, then recompress after approval operations if needed.
