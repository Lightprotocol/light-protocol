## CToken MintToChecked

**discriminator:** 14
**enum:** `InstructionType::CTokenMintToChecked`
**path:** programs/compressed-token/program/src/ctoken/mint_to.rs

**description:**
Mints tokens from a decompressed CMint account to a destination CToken account with decimals validation, fully compatible with SPL Token MintToChecked semantics. Uses pinocchio-token-program to process the mint_to_checked operation which handles balance/supply updates, authority validation, frozen account checks, and decimals validation. After minting, automatically tops up compressible accounts with additional lamports if needed to prevent accounts from becoming compressible during normal operations. Both CMint and destination CToken can receive top-ups based on their current slot and account balance. Supports max_top_up parameter to limit rent top-up costs where 0 means no limit.

Account layouts:
- `CToken` defined in: program-libs/token-interface/src/state/ctoken/ctoken_struct.rs
- `CompressedMint` (CMint) defined in: program-libs/token-interface/src/state/mint/compressed_mint.rs
- `CompressionInfo` extension defined in: program-libs/compressible/src/compression_info.rs

**Instruction data:**
Path: programs/compressed-token/program/src/ctoken/mint_to.rs (function `process_ctoken_mint_to_checked`)
Shared implementation: programs/compressed-token/program/src/ctoken/burn.rs (function `process_ctoken_supply_change_inner`)

Byte layout:
- Bytes 0-7: `amount` (u64, little-endian) - Number of tokens to mint
- Byte 8: `decimals` (u8) - Expected token decimals
- Bytes 9-10: `max_top_up` (u16, little-endian, optional) - Maximum lamports for top-ups in units of 1,000 lamports (e.g., max_top_up=1 means 1,000 lamports, max_top_up=65535 means ~65.5M lamports). 0 = no limit.

Format variants:
- 9 bytes: amount + decimals (legacy, no max_top_up enforcement)
- 11 bytes: amount + decimals + max_top_up

**Accounts:**
1. CMint
   - (writable)
   - The compressed mint account to mint from
   - Validated: mint authority matches authority account
   - Validated: decimals field matches instruction data decimals
   - Supply is increased by mint amount
   - May receive rent top-up if compressible

2. destination CToken
   - (writable)
   - The destination CToken account to mint to
   - Validated: mint field matches CMint pubkey, not frozen
   - Balance is increased by mint amount
   - May receive rent top-up if compressible

3. authority
   - (signer)
   - Mint authority of the CMint account
   - Validated: must sign the transaction
   - If no fee_payer provided: also serves as payer for top-ups (must be writable)
   - If fee_payer provided: readonly (only needs to sign)

4. fee_payer (optional)
   - (signer, writable)
   - Optional separate account to pay for rent top-ups
   - If not provided, authority account pays for top-ups
   - Must have sufficient lamports to cover the top-up amount

**Instruction Logic and Checks:**

1. **Validate minimum accounts:**
   - Require at least 3 accounts (cmint, destination, authority)
   - Return NotEnoughAccountKeys if insufficient

2. **Parse instruction data:**
   - Require at least 9 bytes (amount + decimals)
   - Parse max_top_up from bytes 9-11 if present (11-byte format)
   - Default to 0 (no limit) if only 9 bytes provided (legacy format)
   - Return InvalidInstructionData if length is invalid (not 9 or 11 bytes)

3. **Process mint_to_checked (inline via pinocchio-token-program library):**
   - Call `process_mint_to_checked` with first 9 bytes (amount + decimals)
   - Validates authority signature matches CMint mint authority
   - Validates decimals match CMint's decimals field
   - Checks destination CToken mint matches CMint
   - Checks destination CToken is not frozen
   - Increases destination CToken balance by amount
   - Increases CMint supply by amount
   - Errors are converted from pinocchio errors to ErrorCode variants

4. **Calculate and execute top-up transfers:**
   - Calculate lamports needed for CMint based on compression state (skipped if not compressible)
   - Calculate lamports needed for CToken based on compression state (skipped if no Compressible extension)
   - Validate total against max_top_up budget
   - Transfer lamports from authority to both accounts if needed

**Errors:**

- `ProgramError::NotEnoughAccountKeys` - Less than 3 accounts provided
- `ProgramError::InvalidInstructionData` - Instruction data length is not 9 or 11 bytes
- Pinocchio token errors (converted to ErrorCode variants via `convert_pinocchio_token_error`):
  - `ErrorCode::MintMismatch` (6155) - CToken mint doesn't match CMint
  - `ErrorCode::OwnerMismatch` (6075) - Authority doesn't match CMint mint_authority
  - `ErrorCode::MintDecimalsMismatch` (6166) - Decimals don't match CMint's decimals
  - `ErrorCode::AccountFrozen` (6076) - CToken account is frozen
- `CTokenError::MaxTopUpExceeded` (18043) - Total top-up amount (CMint + CToken) exceeds max_top_up limit
- `CTokenError::MissingPayer` (18061) - Payer account not provided but top-ups are needed

---

## Comparison with SPL Token

### Functional Parity

CToken delegates core logic to `pinocchio_token_program::processor::mint_to_checked::process_mint_to_checked`, which implements SPL Token-compatible mint semantics:
- Authority validation, balance/supply updates, frozen check, mint matching, decimals validation, overflow protection

### CToken-Specific Features

**1. Compressible Top-Up Logic**
Automatically tops up CMint and CToken with rent lamports after minting to prevent accounts from becoming compressible.

**2. max_top_up Parameter**
11-byte instruction format adds `max_top_up` (u16) to limit combined top-up costs. Fails with `MaxTopUpExceeded` (18043) if exceeded.

### Unsupported SPL & Token-2022 Features

**1. No Multisig Support**
**2. No CPI Guard Extension Check**
**3. No Confidential Transfer Mint Extension Check**
