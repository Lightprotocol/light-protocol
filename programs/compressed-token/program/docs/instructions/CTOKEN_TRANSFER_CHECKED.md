## CToken TransferChecked

**discriminator:** 6
**enum:** `InstructionType::CTokenTransferChecked`
**path:** programs/compressed-token/program/src/transfer/checked.rs

### SPL Instruction Format Compatibility

**Important:** This instruction uses the same account layout as SPL Token TransferChecked (source, mint, destination, authority) but has extended instruction data format.

When accounts require rent top-up, lamports are transferred directly from the authority account to the token accounts. The authority must have sufficient lamports to cover the top-up amount.

**Compatibility scenarios:**
- **SPL-compatible:** When using 9-byte instruction data (amount + decimals) with no top-up needed
- **Extended format:** When using 11-byte instruction data (amount + decimals + max_top_up) for compressible accounts

**description:**
Transfers tokens between decompressed ctoken solana accounts with mint decimals validation, fully compatible with SPL Token TransferChecked semantics. Account layout `CToken` is defined in program-libs/ctoken-interface/src/state/ctoken/ctoken_struct.rs. Compression info for rent top-up is defined in program-libs/compressible/src/compression_info.rs. Uses pinocchio-token-program to process the transfer (lightweight SPL-compatible implementation). After the transfer, automatically tops up compressible accounts with additional lamports if needed based on current slot and account balance. Top-up prevents accounts from becoming compressible during normal operations. Supports standard SPL Token transfer features including delegate authority and permanent delegate (multisig not supported). The transfer amount, authority validation, and decimals validation follow SPL Token TransferChecked rules exactly. Validates that mint decimals match the provided decimals parameter. Difference from CTokenTransfer: Requires mint account (4 accounts vs 3) for decimals validation and T22 extension validation.

**Instruction data:**
- **9 bytes (legacy):** amount (u64) + decimals (u8)
- **11 bytes (with max_top_up):** amount (u64) + decimals (u8) + max_top_up (u16)
  - max_top_up: Maximum lamports for top-up operations (0 = no limit)

**Accounts:**
1. source
   - (mutable)
   - The source ctoken account
   - Must have sufficient balance for the transfer
   - Must have same mint as destination
   - May receive rent top-up if compressible
   - If has cached decimals in compressible extension, used for validation

2. mint
   - (immutable)
   - The mint account for the token being transferred
   - Must match source and destination account mints
   - Decimals field must match instruction data decimals parameter
   - Required for T22 extension validation when accounts have restricted extensions

3. destination
   - (mutable)
   - The destination ctoken account
   - Must have same mint as source
   - Must have matching T22 extension markers (pausable, permanent_delegate, transfer_fee, transfer_hook)
   - May receive rent top-up if compressible

4. authority
   - (signer)
   - Owner of the source account or delegate with sufficient allowance
   - Must sign the transaction
   - If is permanent delegate, validated as signer and pinocchio validation is skipped
   - Also serves as payer for top-ups when accounts have compressible extension

**Instruction Logic and Checks:**

1. **Validate minimum accounts:**
   - Require exactly 4 accounts (source, mint, destination, authority)
   - Return NotEnoughAccountKeys if insufficient

2. **Validate instruction data:**
   - Must be at least 9 bytes (amount + decimals)
   - If 11 bytes, parse max_top_up from bytes [9..11]
   - If 9 bytes, set max_top_up = 0 (legacy, no limit)
   - Any other length returns InvalidInstructionData

3. **Parse max_top_up parameter:**
   - 0 = no limit on top-up lamports
   - Non-zero = maximum combined lamports for source + destination top-up
   - Transaction fails if calculated top-up exceeds max_top_up

4. **Process transfer extensions:**
   - Call process_transfer_extensions from shared.rs with source, destination, authority, mint, and max_top_up
   - Validate sender (source account):
     - Deserialize source account (CToken) and extract extension information
     - Validate mint account matches source token's mint field
     - Check for T22 restricted extensions (pausable, permanent_delegate, transfer_fee, transfer_hook, default_account_state)
     - If source has restricted extensions, deserialize and validate mint extensions once:
       - Mint must not be paused
       - Transfer fees must be zero
       - Transfer hooks must have nil program_id
       - Extract permanent delegate if present
     - Validate permanent delegate authority if applicable
     - Cache decimals from compressible extension if has_decimals flag is set
   - Validate recipient (destination account):
     - Deserialize destination account and extract extension information
     - No mint validation for recipient (only sender needs to match mint)
     - Extract T22 extension markers
   - Verify sender and destination have matching T22 extension markers
   - Calculate top-up amounts for both accounts based on compression info:
     - Get current slot from Clock sysvar (lazy loaded once)
     - Get rent exemption from Rent sysvar
     - Call calculate_top_up_lamports for each account
   - Transfer lamports from authority to accounts if top-up needed:
     - Check max_top_up budget if set (non-zero)
     - Execute multi_transfer_lamports atomically
   - Return (signer_is_validated, decimals) tuple

5. **Extract decimals and execute transfer:**
   - Parse amount and decimals from instruction data using unpack_amount_and_decimals
   - If source account has cached decimals in compressible extension (extension_decimals is Some):
     - Validate extension_decimals == instruction decimals parameter
     - Create accounts slice without mint: [source, destination, authority]
     - Call pinocchio process_transfer with expected_decimals = None
     - signer_is_validated flag from permanent delegate check skips redundant owner/delegate validation
   - If no cached decimals (extension_decimals is None):
     - Validate mint account owner is token program
     - Call pinocchio process_transfer with all 4 accounts [source, mint, destination, authority] and expected_decimals = Some(decimals)
     - signer_is_validated flag from permanent delegate check skips redundant owner/delegate validation
   - pinocchio-token-program validates:
     - Source and destination have same mint
     - Mint decimals match provided decimals parameter (when expected_decimals is Some)
     - Authority is owner or delegate with sufficient allowance (unless signer_is_validated is true)
     - Source has sufficient balance
     - Accounts are not frozen
     - Delegate amount is decremented if delegated transfer
   - Transfers amount from source to destination

**Errors:**

- `ProgramError::NotEnoughAccountKeys` (error code: 11) - Less than 4 accounts provided
- `ProgramError::InvalidInstructionData` (error code: 3) - Instruction data is not 9 or 11 bytes, or decimals validation failed
- `ProgramError::MissingRequiredSignature` (error code: 8) - Authority is permanent delegate but not a signer
- `CTokenError::InvalidAccountData` (error code: 18002) - Failed to deserialize CToken account, mint mismatch, or invalid extension data
- `CTokenError::SysvarAccessError` (error code: 18020) - Failed to get Clock or Rent sysvar for top-up calculation
- `CTokenError::MaxTopUpExceeded` (error code: 18043) - Calculated top-up exceeds max_top_up limit
- `ProgramError::InsufficientFunds` (error code: 6) - Source balance less than amount (pinocchio error)
- Pinocchio token errors (converted to ProgramError::Custom):
  - `TokenError::OwnerMismatch` (error code: 4) - Authority is not owner or delegate
  - `TokenError::MintMismatch` (error code: 3) - Source and destination have different mints or mint account mismatch
  - `TokenError::AccountFrozen` (error code: 17) - Source or destination account is frozen
  - `TokenError::InsufficientFunds` (error code: 1) - Delegate has insufficient allowance
  - `TokenError::InvalidMint` (error code: 2) - Mint decimals do not match provided decimals parameter
- `ErrorCode::MintRequiredForTransfer` (error code: 6128) - Account has restricted extensions but mint account not provided
- `ErrorCode::MintPaused` (error code: 6127) - Mint has pausable extension and is currently paused
- `ErrorCode::NonZeroTransferFeeNotSupported` (error code: 6129) - Mint has non-zero transfer fee configured
- `ErrorCode::TransferHookNotSupported` (error code: 6130) - Mint has transfer hook with non-nil program_id

## Comparison with Token-2022

### Functional Parity

CToken TransferChecked provides core compatibility with SPL Token-2022's TransferChecked instruction:

- **Same core semantics**: Transfers tokens from source to destination with authority validation and decimals verification
- **Same account ordering**: source (0), mint (1), destination (2), authority (3)
- **Same instruction data**: amount (u64) + decimals (u8) for the first 9 bytes
- **Same validations**: Mint decimals match, source/destination mints match, sufficient balance, frozen state checks
- **Same authority model**: Supports owner, delegate, and permanent delegate as authority
- **Extension awareness**: Both recognize and validate Token-2022 extensions (pausable, permanent delegate, transfer fee, transfer hook)

### CToken-Specific Features

#### 1. Compressible Top-Up Logic
CToken TransferChecked includes automatic rent top-up for compressible accounts that Token-2022 does not have:

- **Automatic lamport top-up**: Both source and destination accounts receive top-up lamports if they have the Compressible extension and are approaching compressibility
- **Top-up calculation**: Uses `calculate_top_up_lamports()` based on current slot, account balance, and rent exemption threshold
- **Payer**: Authority account pays for top-ups via `multi_transfer_lamports`
- **Budget enforcement**: `max_top_up` parameter (bytes 9-11) limits total lamports for combined source + destination top-up (0 = no limit)
- **Purpose**: Prevents accounts from becoming compressible during normal operations, ensuring continuous availability

**Code Reference**: `programs/compressed-token/program/src/transfer/shared.rs:93-122`

#### 2. Max Top-Up Parameter
CToken supports an optional 11-byte instruction format with max_top_up budget:

- **9 bytes (legacy)**: amount + decimals (max_top_up = 0, no limit)
- **11 bytes (extended)**: amount + decimals + max_top_up (u16)
- **Enforcement**: Transaction fails with `MaxTopUpExceeded` if calculated top-up exceeds budget
- **Token-2022**: Has no equivalent budget parameter

**Code Reference**: `programs/compressed-token/program/src/transfer/checked.rs:57-65`

#### 3. Cached Decimals Optimization
CToken can cache mint decimals in the Compressible extension to skip mint account validation:

- **Cache location**: Stored in Compressible extension via `has_decimals` flag and `decimals()` method
- **When cached**: Uses only 3 accounts [source, destination, authority] and validates decimals against instruction parameter
- **When not cached**: Uses all 4 accounts (includes mint) and delegates decimals check to pinocchio-token-program
- **Benefit**: Reduces account requirements and mint deserialization overhead for compressible accounts
- **Token-2022**: Always requires mint account for decimals validation

**Code Reference**: `programs/compressed-token/program/src/transfer/checked.rs:81-101`

#### 4. Single Account Deserialization
CToken deserializes each account (source, destination) exactly once to extract:

- Token-2022 extension flags (pausable, permanent_delegate, transfer_fee, transfer_hook)
- Compressible extension state for top-up calculation
- Cached decimals if present

Token-2022 deserializes accounts multiple times throughout validation.

**Code Reference**: `programs/compressed-token/program/src/transfer/shared.rs:186-264`

### Missing Features

#### 1. No Multisig Support
- **CToken**: Does not support multisignature authorities. Expects exactly 4 accounts.
- **Token-2022**: Supports M-of-N multisig with additional signer accounts (accounts 4..4+M)
- **Validation**: CToken has no multisig account validation or M-of-N signature checks
- **Impact**: Programs requiring multisig must use Token-2022 accounts or implement custom authority logic

**Token-2022 Reference**: `/home/ananas/dev/token-2022/program/src/processor.rs:1899-1914` (validate_owner function)

#### 2. No TransferFee Handling
- **CToken**: Rejects mints with non-zero transfer fees via `check_mint_extensions`
- **Token-2022**: Calculates epoch-based transfer fees, withholds fees in destination's `TransferFeeAmount` extension
- **Fee calculation**: Token-2022 uses `calculate_epoch_fee(epoch, amount)` with checked arithmetic
- **Fee withholding**: Token-2022 updates `withheld_amount` in destination extension
- **CToken behavior**: `has_transfer_fee` flag is detected but fees must be zero (error: `NonZeroTransferFeeNotSupported`)
- **Credited amount**: CToken always credits full amount (no fee deduction), Token-2022 credits `amount - fee`

**Token-2022 Reference**: `/home/ananas/dev/token-2022/analysis/transfer-checked.md:94-96, 211-222`
**CToken Reference**: `programs/compressed-token/program/src/transfer/shared.rs:245-249` (extension flag detection)

#### 3. No TransferHook Execution
- **CToken**: Rejects mints with transfer hooks that have non-nil program_id
- **Token-2022**: Invokes external hook programs via CPI with transferring flag protection
- **Reentrancy protection**: Token-2022 sets `TransferHookAccount.transferring = true` before CPI, clears after
- **CPI invocation**: Token-2022 calls `spl_transfer_hook_interface::onchain::invoke_execute()`
- **CToken behavior**: `has_transfer_hook` flag is detected but hook program must be nil/zero (error: `TransferHookNotSupported`)
- **Use case limitation**: CToken cannot support custom transfer logic hooks

**Token-2022 Reference**: `/home/ananas/dev/token-2022/analysis/transfer-checked.md:236-270`
**CToken Reference**: `programs/compressed-token/program/src/transfer/shared.rs:250-253` (extension flag detection)

#### 4. No Self-Transfer Optimization
- **CToken**: Processes source and destination independently even when identical
- **Token-2022**: Detects `source_account_info.key == destination_account_info.key` and exits early after validation
- **Token-2022 placement**: Self-transfer check occurs at line 469, AFTER all security validations but BEFORE state modifications
- **Benefit**: Token-2022 saves computation for self-transfers while maintaining security
- **CToken impact**: Self-transfers execute full logic including balance updates and top-ups

**Token-2022 Reference**: `/home/ananas/dev/token-2022/analysis/transfer-checked.md:157-163, 296-304`

#### 5. No Native SOL Support
- **CToken**: Does not support wrapped SOL (native tokens)
- **Token-2022**: Synchronizes SOL lamport balances with token amounts for `is_native()` accounts
- **Token-2022 behavior**: Uses `checked_sub`/`checked_add` on lamports field to match token transfer
- **CToken accounts**: Only support SPL-compatible token accounts, not native SOL wrapping

**Token-2022 Reference**: `/home/ananas/dev/token-2022/analysis/transfer-checked.md:225-234`

#### 6. No Confidential Transfer Support
- **CToken**: Does not check `ConfidentialTransferAccount` extension
- **Token-2022**: Validates `non_confidential_transfer_allowed()` for accounts with confidential extension
- **Token-2022 error**: `NonConfidentialTransfersDisabled` when confidential account blocks non-confidential credits
- **Use case**: Token-2022 supports privacy-preserving transfers with encrypted amounts

**Token-2022 Reference**: `/home/ananas/dev/token-2022/analysis/transfer-checked.md:188-192`

#### 7. No Memo Requirement Support
- **CToken**: Does not validate MemoTransfer extension requirements
- **Token-2022**: Checks `MemoTransfer` extension on both source and destination, ensures memo instruction precedes transfer
- **Token-2022 validation**: Inspects previous sibling instruction for memo program invocation
- **Token-2022 error**: `MissingMemoInPreviousInstruction` when memo required but not present
- **Compliance**: Token-2022 supports regulatory requirements for transaction memos

**Token-2022 Reference**: `/home/ananas/dev/token-2022/analysis/transfer-checked.md:182-186, 325-326`

#### 8. No CPI Guard Support
- **CToken**: Does not check CpiGuard extension
- **Token-2022**: Blocks owner-signed transfers when `CpiGuard.lock_cpi` is enabled and execution is in CPI context
- **Token-2022 validation**: Checks `cpi_guard.lock_cpi.into() && in_cpi() && authority == owner` (lines 402-412)
- **Security**: Prevents CPI Guard bypass even when owner is permanent delegate
- **Token-2022 error**: `CpiGuardTransferBlocked`

**Token-2022 Reference**: `/home/ananas/dev/token-2022/analysis/transfer-checked.md:115-120, 306-307`

#### 9. No NonTransferable Support
- **CToken**: Does not check NonTransferableAccount extension
- **Token-2022**: Prevents all transfers from accounts marked as non-transferable
- **Token-2022 validation**: `source_account.get_extension::<NonTransferableAccount>().is_ok()` check (line 324)
- **Token-2022 error**: `TokenError::NonTransferable`
- **Use case**: Token-2022 supports soulbound/non-transferable tokens

**Token-2022 Reference**: `/home/ananas/dev/token-2022/analysis/transfer-checked.md:62-65`

### Extension Handling Differences

#### Extensions CToken Validates (With Restrictions)

1. **PausableAccount** (account extension)
   - **Detection**: Extracts `has_pausable` flag from source and destination extensions
   - **Validation**: Requires source/destination to have matching pausable flags
   - **Mint check**: Validates mint is not paused via `check_mint_extensions`
   - **Token-2022**: Same validation, checks `PausableConfig.paused.into() == false`
   - **Reference**: `programs/compressed-token/program/src/transfer/shared.rs:239-241`

2. **PermanentDelegateAccount** (account extension)
   - **Detection**: Extracts `has_permanent_delegate` flag from extensions
   - **Validation**: If authority matches permanent delegate pubkey from mint, validates is_signer
   - **Difference**: CToken skips pinocchio validation when permanent delegate is validated (`signer_is_validated = true`)
   - **Token-2022**: Validates permanent delegate via multisig-aware `validate_owner()`
   - **Reference**: `programs/compressed-token/program/src/transfer/shared.rs:242-244, 164-178`

3. **TransferFeeAccount** (account extension)
   - **Detection**: Extracts `has_transfer_fee` flag from extensions
   - **Validation**: Requires mint's `TransferFeeConfig` has zero fees for current epoch
   - **Error**: `NonZeroTransferFeeNotSupported` if fees are configured
   - **Token-2022**: Calculates and withholds fees in destination's `TransferFeeAmount` extension
   - **Reference**: `programs/compressed-token/program/src/transfer/shared.rs:245-249`

4. **TransferHookAccount** (account extension)
   - **Detection**: Extracts `has_transfer_hook` flag from extensions
   - **Validation**: Requires mint's `TransferHook` has nil (zero) program_id
   - **Error**: `TransferHookNotSupported` if hook program is set
   - **Token-2022**: Executes hook via CPI with transferring flag protection
   - **Reference**: `programs/compressed-token/program/src/transfer/shared.rs:250-253`

#### Extension Consistency Enforcement

- **CToken**: Requires source and destination to have matching T22 extension flags (`has_pausable`, `has_permanent_delegate`, `has_transfer_fee`, `has_transfer_hook`)
- **Validation**: Single check comparing all 4 flags via `check_t22_extensions()`
- **Token-2022**: Validates extensions independently based on presence/absence
- **Error**: `InvalidInstructionData` if flags mismatch
- **Purpose**: Ensures both accounts are compatible for transfer operations

**Reference**: `programs/compressed-token/program/src/transfer/shared.rs:32-42, 79`

#### Extensions Not Supported by CToken

- **NonTransferableAccount** - No validation, allows transfers from non-transferable accounts
- **CpiGuard** - No validation, allows CPI transfers even with lock_cpi enabled
- **MemoTransfer** - No validation, does not enforce memo requirements
- **ConfidentialTransferAccount** - No validation, does not handle confidential accounts
- **ImmutableOwner** - Not checked (not relevant to transfers)

### Security Property Comparison

#### Shared Security Properties

1. **Account Ownership Validation**: Both validate source/destination are owned by token program
2. **Frozen State Checks**: Both prevent transfers from/to frozen accounts
3. **Balance Sufficiency**: Both validate source has sufficient balance before transfer
4. **Mint Consistency**: Both validate source/destination have same mint
5. **Decimals Validation**: Both ensure provided decimals match mint decimals
6. **Checked Arithmetic**: Both use checked operations for balance updates to prevent overflow
7. **Authority Validation**: Both support owner, delegate, and permanent delegate authorities

#### CToken-Specific Security

1. **Extension Flag Matching**: CToken enforces source/destination must have identical T22 extension flags
2. **Top-Up Budget Enforcement**: `max_top_up` parameter prevents excessive lamport transfers
3. **Zero-Fee Requirement**: CToken rejects any mint with non-zero transfer fees (fail-safe)
4. **Nil Hook Requirement**: CToken rejects any mint with non-nil transfer hook program_id (fail-safe)
5. **Single Deserialization**: Each account deserialized exactly once reduces attack surface

#### Token-2022-Specific Security

1. **Self-Transfer Validation Ordering**: Self-transfer check occurs AFTER all security validations but BEFORE state modifications (prevents bypass)
2. **CPI Guard Bypass Prevention**: Explicitly blocks CPI transfers even when owner is permanent delegate
3. **Reentrancy Protection**: Transferring flag prevents recursive calls during transfer hook execution
4. **Multisig Validation**: M-of-N signature validation for multisig authorities
5. **Non-Transferable Enforcement**: Blocks all transfers from soulbound tokens
6. **Memo Compliance**: Ensures regulatory requirements via memo instruction validation
7. **Native SOL Synchronization**: Prevents lamport/token desynchronization for wrapped SOL

#### Known Vulnerability Mitigations

Both CToken and Token-2022 mitigate:

- **Supply Inflation Bugs**: Balance checks before state changes + checked arithmetic
- **Mint Mismatch**: Triple validation (source-mint, source-dest, decimals)
- **Account Ordering Issues**: Explicit account extraction with typed unpacking
- **Overflow Vulnerabilities**: All arithmetic uses checked variants

Token-2022 additionally mitigates:

- **CPI Guard Bypass**: Explicit check for `authority == owner && lock_cpi && in_cpi()` (Certora-2024 audit finding)
- **Transfer Fee Overflow**: Fee calculation returns Option with explicit overflow handling
- **Reentrancy Attacks**: Transferring flag prevents hook reentrancy

**Token-2022 Reference**: `/home/ananas/dev/token-2022/analysis/transfer-checked.md:348-370`
