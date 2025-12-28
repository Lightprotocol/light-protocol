## CToken MintTo

**discriminator:** 7
**enum:** `InstructionType::CTokenMintTo`
**path:** programs/compressed-token/program/src/ctoken_mint_to.rs

**description:**
Mints tokens from a decompressed CMint account to a destination CToken account, fully compatible with SPL Token mint_to semantics. Uses pinocchio-token-program to process the mint_to operation which handles balance/supply updates, authority validation, and frozen account checks. After minting, automatically tops up compressible accounts with additional lamports if needed to prevent accounts from becoming compressible during normal operations. Both CMint and destination CToken can receive top-ups based on their current slot and account balance. Supports max_top_up parameter to limit rent top-up costs where 0 means no limit. Instruction data is backwards-compatible with two formats: 8-byte format for legacy compatibility without max_top_up enforcement and 10-byte format with max_top_up. This instruction only works with CMints (compressed mints). CMints do not support restricted Token-2022 extensions (Pausable, TransferFee, TransferHook, PermanentDelegate, DefaultAccountState) - only TokenMetadata is allowed.

Account layouts:
- `CToken` defined in: program-libs/ctoken-interface/src/state/ctoken/ctoken_struct.rs
- `CompressedMint` (CMint) defined in: program-libs/ctoken-interface/src/state/mint/compressed_mint.rs
- `CompressionInfo` extension defined in: program-libs/compressible/src/compression_info.rs

**Instruction data:**
Path: programs/compressed-token/program/src/ctoken_mint_to.rs (lines 10-47)

Byte layout:
- Bytes 0-7: `amount` (u64, little-endian) - Number of tokens to mint
- Bytes 8-9: `max_top_up` (u16, little-endian, optional) - Maximum lamports for top-ups combined, 0 = no limit

Format variants:
- 8-byte format: amount only, no max_top_up enforcement
- 10-byte format: amount + max_top_up

**Accounts:**
1. CMint
   - (writable)
   - The compressed mint account to mint from
   - Validated: mint authority matches authority account
   - Supply is increased by mint amount
   - May receive rent top-up if compressible

2. destination CToken
   - (writable)
   - The destination CToken account to mint to
   - Validated: mint field matches CMint pubkey, not frozen
   - Balance is increased by mint amount
   - May receive rent top-up if compressible

3. authority
   - (signer, writable when top-ups needed)
   - Mint authority of the CMint account
   - Validated: must sign the transaction
   - Also serves as payer for rent top-ups if needed

**Instruction Logic and Checks:**

1. **Validate minimum accounts:**
   - Require at least 3 accounts (cmint, destination, authority)
   - Return NotEnoughAccountKeys if insufficient

2. **Parse instruction data:**
   - Require at least 8 bytes for amount
   - Parse max_top_up from bytes 8-10 if present (10-byte format)
   - Default to 0 (no limit) if only 8 bytes provided (legacy format)
   - Return InvalidInstructionData if length is invalid (not 8 or 10 bytes)

3. **Process SPL mint_to via pinocchio-token-program:**
   - Call `process_mint_to` with first 8 bytes (amount only)
   - Validates authority signature matches CMint mint authority
   - Checks destination CToken mint matches CMint
   - Checks destination CToken is not frozen
   - Increases destination CToken balance by amount
   - Increases CMint supply by amount
   - Errors are converted from pinocchio errors to ProgramError::Custom

4. **Calculate top-up requirements:**
   For both CMint and destination CToken accounts:

   a. **Deserialize account using zero-copy:**
      - CMint: Use `CompressedMint::zero_copy_at`
      - CToken: Use `CToken::zero_copy_at_checked`
      - Access compression info directly from embedded field (all accounts now have compression embedded)

   b. **Calculate top-up amount:**
      - Get current slot from Clock sysvar (lazy loaded, only if needed)
      - Get rent exemption from Rent sysvar
      - Call `calculate_top_up_lamports` which:
        - Checks if account is compressible
        - Calculates rent deficit if any
        - Adds configured lamports_per_write amount
        - Returns 0 if account is well-funded

   c. **Track lamports budget:**
      - Initialize budget to max_top_up + 1 (allowing exact match)
      - Subtract CMint top-up amount from budget
      - Subtract CToken top-up amount from budget
      - If budget reaches 0 and max_top_up is not 0, fail with MaxTopUpExceeded

5. **Execute top-up transfers:**
   - Skip if no accounts need top-up (both amounts are 0)
   - Use authority account (third account) as funding source
   - Execute multi_transfer_lamports to top up both accounts atomically
   - Update account lamports balances

**Errors:**

- `ProgramError::NotEnoughAccountKeys` (error code: 11) - Less than 3 accounts provided
- `ProgramError::InvalidInstructionData` (error code: 3) - Instruction data length is not 8 or 10 bytes
- Pinocchio token errors (converted to ProgramError::Custom):
  - `TokenError::MintMismatch` (error code: 3) - CToken mint doesn't match CMint
  - `TokenError::OwnerMismatch` (error code: 4) - Authority doesn't match CMint mint_authority
  - `TokenError::AccountFrozen` (error code: 17) - CToken account is frozen
- `CTokenError::CMintDeserializationFailed` (error code: 18047) - Failed to deserialize CMint account using zero-copy
- `CTokenError::InvalidAccountData` (error code: 18002) - Failed to deserialize CToken account or calculate top-up amount
- `CTokenError::SysvarAccessError` (error code: 18020) - Failed to get Clock or Rent sysvar for top-up calculation
- `CTokenError::MaxTopUpExceeded` (error code: 18043) - Total top-up amount (CMint + CToken) exceeds max_top_up limit

---

## Comparison with Token-2022

This section compares CToken MintTo with Token-2022's MintTo and MintToChecked instructions.

### Functional Parity

CToken MintTo maintains core compatibility with Token-2022's MintTo instruction:

- **Authority validation:** Both require mint authority signature and validate against the mint's configured mint_authority
- **Balance updates:** Both increase destination account balance and mint supply by the specified amount
- **Frozen account checks:** Both prevent minting to frozen accounts
- **Mint matching:** Both validate that destination account's mint field matches the mint account
- **Overflow protection:** Both check for arithmetic overflow when adding to balances and supply
- **Fixed supply enforcement:** Both fail if mint_authority is set to None (supply is fixed)

### CToken-Specific Features

CToken MintTo extends Token-2022 functionality with compression-specific features:

**1. Compressible Top-Up Logic**

After minting, CToken MintTo automatically replenishes lamports for compressible accounts to prevent premature compression:

- **Dual account top-up:** Both CMint and destination CToken may receive rent top-ups in a single transaction
- **Compressibility checks:** Uses `calculate_top_up_lamports` to determine if accounts need funding based on:
  - Current slot vs last_compressible_slot
  - Account lamport balance vs rent exemption threshold
  - Configured lamports_per_write amount
- **Automatic funding:** Authority account serves as payer for all top-ups
- **Zero-copy access:** Uses zero-copy deserialization to read compression info directly from embedded fields without full account deserialization

**2. Max Top-Up Parameter**

CToken MintTo includes a `max_top_up` parameter to control rent costs:

- **Budget enforcement:** Limits combined lamports spent on CMint + CToken top-ups
- **Value 0 = unlimited:** Setting max_top_up to 0 means no spending limit
- **Backwards compatibility:** Supports 8-byte format (amount only, no limit) and 10-byte format (amount + max_top_up)
- **Fails on overflow:** Returns MaxTopUpExceeded error if total top-up exceeds budget
- **Prevents DoS:** Protects authority account from unexpected lamport drainage

**3. Authority Account Mutability**

- **Token-2022:** Authority account is read-only (signature verification only)
- **CToken:** Authority account must be writable when top-ups are needed (serves as payer)

### Missing Token-2022 Features

**1. No Multisig Support**

- **Token-2022:** Supports multisig authorities via additional signer accounts (accounts 3..3+M)
- **CToken:** Does not support multisig authorities - only single signer supported
- **Implication:** CToken MintTo expects exactly 3 accounts; Token-2022 accepts 3+ for multisig

**2. No MintToChecked Variant**

- **Token-2022:** Provides MintToChecked instruction that validates decimals parameter against mint
- **CToken:** Does not implement decimals validation in CToken MintTo
- **Token-2022 MintToChecked behavior:**
  - Instruction data: 10 bytes (discriminator + amount + decimals)
  - Validation: `expected_decimals != mint.base.decimals` returns MintDecimalsMismatch error
  - Use case: Prevents minting with incorrect decimal assumptions in offline/hardware wallet scenarios
- **CToken workaround:** Clients must validate decimals independently before calling CToken MintTo

### Extension Handling

CToken MintTo only operates on CMints, which do not support restricted extensions:

- **CMints only support TokenMetadata extension** - no Pausable, TransferFee, TransferHook, PermanentDelegate, or DefaultAccountState
- **No extension checks needed** - CMints cannot have these extensions, so no validation is required
- **Compressible extension (CToken-specific):** Always present in CMint and CToken accounts as embedded field, accessed via zero-copy

### Security Notes

**Shared Security Properties:**

- Both validate authority signature before state changes
- Both check for account ownership by token program
- Both prevent overflow in balance/supply arithmetic
- Both prevent minting to frozen accounts

**CToken-Specific Security Considerations:**

1. **Authority lamport drainage:** Authority must have sufficient lamports for top-ups; use max_top_up to limit exposure
2. **Top-up atomicity:** If top-up fails (insufficient authority balance), entire instruction fails - no partial minting
3. **Compressibility timing:** Top-ups are calculated based on current slot and account state; accounts may still become compressible after minting if not topped up
4. **No multisig protection:** Single authority compromise affects all minting; Token-2022 multisig provides defense in depth

**Token-2022-Specific Security Considerations:**

1. **Extension-based restrictions:** NonTransferable, PausableConfig, and ConfidentialMintBurn extensions add security controls not enforced in CToken MintTo
2. **Decimals validation (MintToChecked):** Prevents decimal precision errors in offline transaction construction

### Summary Table

| Feature | Token-2022 MintTo | Token-2022 MintToChecked | CToken MintTo |
|---------|-------------------|--------------------------|---------------|
| Instruction data | 8 bytes (amount) | 10 bytes (amount + decimals) | 8 or 10 bytes (amount + optional max_top_up) |
| Multisig support | Yes | Yes | No |
| Decimals validation | No | Yes | No |
| Automatic rent top-up | No | No | Yes (compressible accounts) |
| Top-up budget control | N/A | N/A | Yes (max_top_up) |
| Authority account | Read-only | Read-only | Writable (when top-ups needed) |
| Extension checks | NonTransferable, PausableConfig, ConfidentialMintBurn | Same as MintTo | None (CMints don't support restricted extensions) |
| Account count | 3+ (multisig) | 3+ (multisig) | Exactly 3 |
| Backwards compatibility | N/A | N/A | 8-byte format (legacy) and 10-byte format (with max_top_up) |
