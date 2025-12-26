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
Path: programs/compressed-token/program/src/ctoken_approve_revoke.rs (lines 58-82)

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

4. **Process SPL revoke:**
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

## Comparison with Token-2022

### Functional Parity

CToken Revoke maintains functional parity with Token-2022 for the core revoke operation:

1. **Delegate Clearing**: Both implementations atomically clear the `delegate` field and `delegated_amount` to zero
2. **Owner Authority**: Both require the token account owner to sign the transaction
3. **Account State Validation**: Both validate that the source account is properly initialized and owned by the token program
4. **Frozen Account Handling**: Both prevent revoke operations on frozen accounts (enforced by pinocchio-token-program)
5. **Signer Validation**: Both ensure the authority account is a transaction signer

### CToken-Specific Features

CToken Revoke adds compression-aware functionality not present in Token-2022:

1. **Compressible Top-Up Logic**: Automatically tops up accounts with the Compressible extension to prevent them from becoming compressible during normal operations
   - Calculates required lamports based on rent exemption and compression threshold
   - Transfers lamports from owner (payer) to source account via CPI
   - Uses Clock and Rent sysvars to determine compressibility

2. **max_top_up Parameter**: Enforces transaction failure if the calculated top-up exceeds the specified limit
   - `max_top_up = 0` means no limit (legacy behavior)
   - Prevents unexpected lamport transfers during revoke operations
   - Returns `CTokenError::MaxTopUpExceeded` if budget exceeded

3. **Backwards-Compatible Instruction Data**:
   - 0 bytes: Legacy format (no max_top_up enforcement)
   - 2 bytes: New format with max_top_up parameter

### Missing Features

CToken Revoke does NOT implement the following Token-2022 features:

1. **Multisignature Support**: Token-2022 supports M-of-N multisig accounts as the authority
   - Token-2022 validates multisig signers and enforces threshold requirements
   - CToken only supports single-signature owner authority
   - Account requirements: Token-2022 requires additional signer accounts for multisig (2..2+M accounts)

2. **Dual Authority Model**: Token-2022 allows BOTH the account owner AND the current delegate to revoke delegation
   - Token-2022 implementation (lines 637-649 in processor.rs):
     ```rust
     Self::validate_owner(
         program_id,
         match &source_account.base.delegate {
             PodCOption {
                 option: PodCOption::<Pubkey>::SOME,
                 value: delegate,
             } if authority_info.key == delegate => delegate,
             _ => &source_account.base.owner,
         },
         authority_info,
         // ...
     )
     ```
   - CToken only accepts the owner as authority (account index 1)
   - Use case: In Token-2022, delegates can voluntarily relinquish their own authority

3. **No CPI Guard Extension Check**: Token-2022 does not check CPI Guard for Revoke (intentional design)
   - CToken similarly has no CPI Guard check (delegates to pinocchio-token-program)
   - Note: Token-2022 Approve DOES check CPI Guard and blocks approve during CPI if enabled

### Extension Handling Differences

**Token-2022 Extension Interactions:**
- No explicit extension checks in Revoke
- CPI Guard: Not checked (Revoke can be called via CPI even with CpiGuard enabled)
- Non-Transferable: Works on non-transferable accounts (no tokens moved)
- Transfer Hooks: No interaction (no token transfer occurs)
- Permanent Delegate: No conflict (permanent delegate is separate from regular delegate)

**CToken Extension Handling:**
- Compressible extension: Explicitly processed for rent top-up
- No other extension-specific logic (delegates to pinocchio-token-program for base validation)

### Security Property Comparison

**Shared Security Properties:**
1. **Program Ownership Validation**: Both validate source account is owned by token program
2. **Initialization Check**: Both ensure account is initialized before processing
3. **Frozen Account Protection**: Both block revoke on frozen accounts
4. **Authority Key Matching**: Both verify authority signature matches expected owner
5. **Atomic State Updates**: Both clear delegate and delegated_amount together
6. **No Balance Checks**: Both are pure authority operations (no token balance validation)

**CToken-Specific Security:**
1. **Rent Protection**: max_top_up parameter prevents unexpected lamport transfers
2. **Compressibility Prevention**: Ensures accounts remain above compression threshold after operation
3. **Zero-Copy Validation**: Uses zero-copy deserialization for CToken account structure

**Token-2022-Specific Security:**
1. **Multisig Validation**: Enforces M-of-N signature requirements for multisig authorities
2. **Duplicate Signer Prevention**: Prevents counting same signer multiple times in multisig
3. **Delegate Self-Revocation**: Allows delegate to remove their own authority (not available in CToken)

### Implementation Differences

**Token-2022 (lines 624-654 in processor.rs):**
- Direct processor implementation
- Flexible authority selection (owner OR delegate)
- No additional lamport transfers
- No instruction data (unit variant)

**CToken (programs/compressed-token/program/src/ctoken_approve_revoke.rs):**
- Wrapper around pinocchio-token-program's process_revoke
- Owner-only authority model
- Pre-processes compressible top-up before delegating to SPL logic
- Optional instruction data for max_top_up parameter (0 or 2 bytes)

### Use Case Implications

1. **Standard Token Operations**: CToken Revoke provides identical functionality for non-compressible accounts
2. **Compression-Aware Applications**: CToken's top-up logic prevents surprise account compression
3. **Multisig Wallets**: Not supported in CToken (use Token-2022 for multisig requirements)
4. **Delegate Self-Revocation**: Not available in CToken (only owner can revoke)
5. **Budget-Constrained Transactions**: max_top_up parameter enables precise lamport budget control

### Overall Risk Assessment

**CToken Revoke**: Low risk. Well-secured with comprehensive validation and compression-specific protections. Missing multisig support reduces attack surface but limits flexibility for advanced wallet architectures.

**Token-2022 Revoke**: Low risk. Comprehensive validation with additional multisig support and dual authority model. CPI Guard intentionally not enforced to preserve revoke functionality in all contexts.
