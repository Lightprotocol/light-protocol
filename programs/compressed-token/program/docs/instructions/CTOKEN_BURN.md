## CToken Burn

**discriminator:** 8
**enum:** `InstructionType::CTokenBurn`
**path:** programs/compressed-token/program/src/ctoken_burn.rs

**description:**
Burns tokens from a decompressed CToken account and decreases the CMint supply, fully compatible with SPL Token burn semantics. Account layout `CToken` is defined in `program-libs/ctoken-interface/src/state/ctoken/ctoken_struct.rs`. Account layout `CompressedMint` (CMint) is defined in `program-libs/ctoken-interface/src/state/mint/compressed_mint.rs`. Extension layout `CompressionInfo` is defined in `program-libs/compressible/src/compression_info.rs` and is embedded in both CToken and CMint structs. Uses pinocchio-token-program to process the burn (handles balance/supply updates, authority check, frozen check). After the burn, automatically tops up compressible accounts with additional lamports if needed. Top-up is calculated for both CMint and source CToken based on current slot and account balance. Top-up prevents accounts from becoming compressible during normal operations. Enforces max_top_up limit if provided (transaction fails if exceeded). Account order is REVERSED from mint_to instruction: [source_ctoken, cmint, authority] vs mint_to's [cmint, destination_ctoken, authority]. Supports max_top_up parameter to limit rent top-up costs (0 = no limit). Instruction data is backwards-compatible: 8-byte format (legacy, no max_top_up enforcement) and 10-byte format (with max_top_up). This instruction only works with CMints (compressed mints). CMints do not support restricted Token-2022 extensions (Pausable, TransferFee, TransferHook, PermanentDelegate, DefaultAccountState) - only TokenMetadata is allowed. To burn tokens from T22 mints with restricted extensions, use Transfer2 with decompress mode to convert to SPL tokens first, then burn via SPL Token-2022.

**Instruction data:**

Format 1 (8 bytes, legacy):
- Bytes 0-7: `amount` (u64, little-endian) - Number of tokens to burn
- No max_top_up enforcement (effectively unlimited)

Format 2 (10 bytes):
- Bytes 0-7: `amount` (u64, little-endian) - Number of tokens to burn
- Bytes 8-9: `max_top_up` (u16, little-endian) - Maximum lamports for combined CMint + CToken top-ups (0 = no limit)

**Accounts:**
1. source CToken
   - (mutable)
   - The CToken account to burn from
   - Must have sufficient balance for the burn
   - May receive rent top-up if compressible
   - Must not be frozen

2. CMint
   - (mutable)
   - The compressed mint account
   - Supply is decreased by burn amount
   - May receive rent top-up if compressible

3. authority
   - (signer)
   - Owner of the source CToken account
   - Must sign the transaction
   - Also serves as payer for rent top-ups if needed

**Instruction Logic and Checks:**

1. **Validate minimum accounts:**
   - Require at least 3 accounts (source CToken, CMint, authority/payer)
   - Return NotEnoughAccountKeys if insufficient

2. **Parse instruction data:**
   - Require at least 8 bytes for amount
   - Parse max_top_up:
     - If instruction_data.len() == 8: max_top_up = 0 (no limit, legacy format)
     - If instruction_data.len() == 10: parse u16 from bytes 8-9 as max_top_up
     - Otherwise: return InvalidInstructionData

3. **Process SPL burn via pinocchio-token-program:**
   - Call `process_burn` with first 8 bytes (amount only)
   - Validates authority signature matches source CToken owner
   - Checks source CToken balance is sufficient for burn amount
   - Checks source CToken is not frozen
   - Decreases source CToken balance by amount
   - Decreases CMint supply by amount
   - Errors are converted from pinocchio errors to ProgramError::Custom

4. **Calculate and execute top-up transfers:**
   Called via `calculate_and_execute_compressible_top_ups(cmint, ctoken, payer, max_top_up)`:

   a. **Initialize transfer array and budget:**
      - Create Transfer array for [cmint, ctoken] with amounts initialized to 0
      - Initialize lamports_budget to max_top_up + 1 (allowing exact match when total == max_top_up)

   b. **Calculate CMint top-up:**
      - Borrow CMint data and deserialize using `CompressedMint::zero_copy_at`
      - Access compression info directly from mint.base.compression (embedded in all CMints)
      - Lazy load Clock sysvar for current_slot and Rent sysvar if not yet loaded (current_slot == 0)
      - Call `compression.calculate_top_up_lamports(data_len, current_slot, lamports, rent_exemption)`
      - Subtract calculated top-up from lamports_budget

   c. **Calculate CToken top-up:**
      - Borrow CToken data and deserialize using `CToken::zero_copy_at_checked`
      - Access compression info directly from token.compression (embedded in all CTokens)
      - Lazy load Clock sysvar for current_slot and Rent sysvar if not yet loaded (current_slot == 0)
      - Call `compression.calculate_top_up_lamports(data_len, current_slot, lamports, rent_exemption)`
      - Subtract calculated top-up from lamports_budget

   d. **Validate budget:**
      - If no compressible accounts were found (current_slot == 0), exit early
      - If both top-up amounts are 0, exit early
      - If max_top_up != 0 and lamports_budget == 0, fail with MaxTopUpExceeded

   e. **Execute transfers:**
      - Call `multi_transfer_lamports(payer, &transfers)` to atomically transfer lamports
      - Updates account balances for both CMint and CToken if needed

**Errors:**

- `ProgramError::NotEnoughAccountKeys` (error code: 11) - Less than 3 accounts provided
- `ProgramError::InvalidInstructionData` (error code: 3) - Instruction data length is not 8 or 10 bytes
- `ProgramError::InsufficientFunds` (error code: 6) - Source CToken balance less than burn amount (from pinocchio burn), or payer has insufficient funds for top-up transfers
- `ProgramError::ArithmeticOverflow` (error code: 24) - Overflow when calculating total top-up amount
- Pinocchio token errors (converted to ProgramError::Custom):
  - `TokenError::OwnerMismatch` (error code: 4) - Authority is not owner or delegate
  - `TokenError::MintMismatch` (error code: 3) - CToken mint doesn't match CMint
  - `TokenError::AccountFrozen` (error code: 17) - CToken account is frozen
- `CTokenError::CMintDeserializationFailed` (error code: 18047) - Failed to deserialize CMint account using zero-copy
- `CTokenError::InvalidAccountData` (error code: 18002) - Account data length is too small, calculate top-up failed, or invalid account format
- `CTokenError::InvalidAccountState` (error code: 18036) - CToken account is not initialized
- `CTokenError::InvalidAccountType` (error code: 18053) - Account is not a CToken account type
- `CTokenError::SysvarAccessError` (error code: 18020) - Failed to get Clock or Rent sysvar for top-up calculation
- `CTokenError::MaxTopUpExceeded` (error code: 18043) - Total top-up amount (CMint + CToken) exceeds max_top_up limit

## Comparison with Token-2022

CToken Burn implements similar core functionality to SPL Token-2022's Burn instruction, with key differences to support Light Protocol's compressed token model.

### Functional Parity

Both implementations share these core behaviors:

1. **Balance/Supply Updates**: Decrease token account balance and mint supply by burn amount
2. **Authority Validation**: Verify owner signature or delegate authority using multisig support
3. **Account State Checks**:
   - Frozen account check (fails if account is frozen)
   - Native mint check (native SOL burning not supported)
   - Mint mismatch validation (account must belong to specified mint)
   - Insufficient funds check (account must have sufficient balance)
4. **Delegate Handling**: Support for burning via delegate with delegated amount tracking
5. **Permanent Delegate**: Honor permanent delegate authority if configured on mint
6. **BurnChecked Variant**: Both support decimal validation (Token-2022's BurnChecked, CToken's optional decimals parameter in pinocchio burn)

**Implementation Note**: CToken Burn delegates core burn logic to `pinocchio_token_program::processor::burn::process_burn`, which implements SPL-compatible burn semantics including all checks above.

### CToken-Specific Features

#### 1. Compressible Top-Up Logic

CToken Burn automatically tops up compressible accounts with rent lamports after burning:

```rust
// After burn, calculate and execute top-ups for both CMint and CToken
calculate_and_execute_compressible_top_ups(cmint, ctoken, payer, max_top_up)
```

**Top-up flow:**
1. Calculate lamports needed for CMint based on compression state (current slot, balance, data length)
2. Calculate lamports needed for CToken based on compression state
3. Validate total against `max_top_up` budget
4. Transfer lamports from payer (authority account) to both accounts if needed

**Purpose**: Prevents accounts from becoming compressible during normal operations by maintaining sufficient rent balance.

#### 2. max_top_up Parameter

Instruction data supports two formats:
- **Legacy (8 bytes)**: `amount` only, no top-up limit (max_top_up = 0)
- **Extended (10 bytes)**: `amount` + `max_top_up` (u16), enforces combined CMint+CToken top-up limit

```rust
let max_top_up = match instruction_data.len() {
    8 => 0u16,      // no limit
    10 => u16::from_le_bytes(instruction_data[8..10])?,
    _ => return Err(InvalidInstructionData),
};
```

If `max_top_up != 0` and total required lamports exceed limit, transaction fails with `MaxTopUpExceeded` (18043).

### Missing Features (vs Token-2022)

#### 1. No Multisig Support

**Token-2022**: Supports multisignature authorities with M-of-N signature validation
```
Accounts (multisig variant):
0. source account (writable)
1. mint (writable)
2. multisig authority account
3..3+M. signer accounts (M signers required)
```

**CToken Burn**: Only supports single-signer authority
```
Accounts:
0. source CToken (writable)
1. CMint (writable)
2. authority (signer, also payer)
```

**Reason**: Pinocchio burn implementation handles multisig through `validate_owner()`, but CToken Burn only provides 3 accounts minimum. Multisig would require additional signer accounts and explicit multisig account validation.

#### 2. No BurnChecked Instruction Variant

**Token-2022**: Separate `BurnChecked` instruction (discriminator 15) with explicit decimals parameter in instruction data
```rust
BurnChecked {
    amount: u64,
    decimals: u8,  // Must match mint decimals
}
```

**CToken Burn**: Single instruction (discriminator 8) with optional decimals validation in pinocchio layer
```rust
// Pinocchio burn signature:
pub fn process_burn(
    accounts: &[AccountInfo],
    instruction_data: &[u8],  // 8 bytes: amount only
) -> Result<(), TokenError>
```

**Implication**: CToken Burn relies on pinocchio's internal validation. No explicit decimals check in CToken instruction data format. If decimals validation is needed, it must be added to instruction data structure.

#### 3. No NonTransferableTokens Extension Check

**Token-2022**: Does NOT check `NonTransferableAccount` extension during burn (burning non-transferable tokens is allowed)
```rust
// Token-2022 allows burning non-transferable tokens
// Only transfers are blocked for NonTransferableAccount
if source_account.get_extension::<NonTransferableAccount>().is_ok() {
    return Err(TokenError::NonTransferable.into());  // Only in transfer
}
```

**CToken Burn**: No check for `NonTransferableAccount` extension (matches Token-2022 behavior)

**Why allowed**: Burning reduces supply and eliminates tokens - doesn't violate non-transferable constraint since tokens aren't moving to another account.

### Extension Handling

CToken Burn only operates on CMints, which do not support restricted extensions:

- **CMints only support TokenMetadata extension** - no Pausable, TransferFee, TransferHook, PermanentDelegate, or DefaultAccountState
- **No extension checks needed** - CMints cannot have these extensions, so no validation is required
- **For T22 mints with restricted extensions**: Use Transfer2 (decompress) to convert to SPL tokens, then burn via SPL Token-2022

### Security Notes

#### 1. Account Order Reversed from MintTo

```
CToken MintTo: [cmint, destination_ctoken, authority]
CToken Burn:   [source_ctoken, cmint, authority]
```

**Reason**: SPL Token convention - source account first for burn, destination first for mint. CToken follows this pattern for pinocchio compatibility.

#### 2. Top-Up Payer is Authority

Unlike mint_to where payer is a separate account, burn uses the authority (signer) as payer for rent top-ups:

```rust
let payer = accounts.get(2).ok_or(ProgramError::NotEnoughAccountKeys)?;  // Same as authority
```

**Implication**: Burning tokens may require additional lamports from the authority's account if CMint/CToken are compressible and need top-up.

#### 3. Pinocchio Error Conversion

```rust
process_burn(accounts, &instruction_data[..8])
    .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?;
```

Pinocchio errors are converted to `ProgramError::Custom`. Common TokenError codes:
- `TokenError::OwnerMismatch` (4)
- `TokenError::MintMismatch` (3)
- `TokenError::AccountFrozen` (17)
- `TokenError::InsufficientFunds` (1)

#### 4. No Extension Validation Before Pinocchio Call

CToken Burn does NOT call `check_mint_extensions()` before burning. Extension checks (PausableConfig, PermanentDelegate) are handled internally by pinocchio burn logic.

**Contrast with Transfer2/CTokenTransfer**: Those instructions explicitly call `check_mint_extensions()` to validate TransferFeeConfig, TransferHook, PausableConfig, and extract PermanentDelegate.

**Risk**: If future Token-2022 extensions require pre-burn validation, CToken Burn would need to add explicit extension checks before calling pinocchio.
