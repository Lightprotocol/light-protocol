## CToken MintTo

**discriminator:** 7
**enum:** `InstructionType::CTokenMintTo`
**path:** programs/compressed-token/program/src/ctoken/mint_to.rs

**description:**
Mints tokens from a decompressed CMint account to a destination CToken account, fully compatible with SPL Token mint_to semantics. Uses pinocchio-token-program to process the mint_to operation which handles balance/supply updates, authority validation, and frozen account checks. After minting, automatically tops up compressible accounts with additional lamports if needed to prevent accounts from becoming compressible during normal operations. Both CMint and destination CToken can receive top-ups based on their current slot and account balance. Supports max_top_up parameter to limit rent top-up costs where u16::MAX means no limit, 0 means no top-ups allowed. Instruction data is backwards-compatible with two formats: 8-byte format for legacy compatibility without max_top_up enforcement and 10-byte format with max_top_up. This instruction only works with CMints (compressed mints). CMints do not support restricted Token-2022 extensions (Pausable, TransferFee, TransferHook, PermanentDelegate, DefaultAccountState) - only TokenMetadata is allowed.

Account layouts:
- `CToken` defined in: program-libs/token-interface/src/state/ctoken/ctoken_struct.rs
- `CompressedMint` (CMint) defined in: program-libs/token-interface/src/state/mint/compressed_mint.rs
- `CompressionInfo` extension defined in: program-libs/compressible/src/compression_info.rs

**Instruction data:**
Path: programs/compressed-token/program/src/ctoken/mint_to.rs (see `process_ctoken_mint_to` function)

Byte layout:
- Bytes 0-7: `amount` (u64, little-endian) - Number of tokens to mint
- Bytes 8-9: `max_top_up` (u16, little-endian, optional) - Maximum lamports for top-ups in units of 1,000 lamports (e.g., max_top_up=1 means 1,000 lamports, max_top_up=65535 means ~65.5M lamports). u16::MAX = no limit, 0 = no top-ups allowed.

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
   - Require at least 8 bytes for amount
   - Parse max_top_up from bytes 8-10 if present (10-byte format)
   - Default to u16::MAX (no limit) if only 8 bytes provided (legacy format)
   - Return InvalidInstructionData if length is invalid (not 8 or 10 bytes)

3. **Process mint_to (inline via pinocchio-token-program library):**
   - Call `process_mint_to` with first 8 bytes (amount only)
   - Validates authority signature matches CMint mint authority
   - Checks destination CToken mint matches CMint
   - Checks destination CToken is not frozen
   - Increases destination CToken balance by amount
   - Increases CMint supply by amount
   - Errors are converted from pinocchio errors to ProgramError::Custom

4. **Calculate top-up requirements:**
   For both CMint and destination CToken accounts:

   a. **Access CompressionInfo using optimized byte access:**
      - CMint: Use `cmint_top_up_lamports_from_account_info` which reads CompressionInfo at fixed byte offset (166)
      - CToken: Use `top_up_lamports_from_account_info_unchecked` which reads CompressionInfo at fixed byte offset (176)
      - Returns None if account lacks CompressionInfo (CMint without compression, or CToken without Compressible extension as first extension)

   b. **Calculate top-up amount:**
      - Get current slot from Clock sysvar (lazy loaded on first compressible account)
      - Uses stored rent_exemption_paid from CompressionInfo (not Rent sysvar)
      - Call `calculate_top_up_lamports` which:
        - Checks if account is compressible
        - Calculates rent deficit if any
        - Adds configured lamports_per_write amount
        - Returns 0 if account is well-funded

   c. **Track lamports budget:**
      - Initialize budget to max_top_up.saturating_add(1) (allowing exact match)
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
- `CTokenError::MaxTopUpExceeded` (error code: 18043) - Total top-up amount (CMint + CToken) exceeds max_top_up limit
- `CTokenError::MissingPayer` (error code: 18061) - Payer account missing when top-ups are needed

---

## Comparison with SPL Token

### Functional Parity

CToken delegates core logic to `pinocchio_token_program::processor::mint_to::process_mint_to`, which implements SPL Token-compatible mint semantics:
- Authority validation, balance/supply updates, frozen check, mint matching, overflow protection
- **MintToChecked:** CToken implements CTokenMintToChecked (discriminator: 14) with full decimals validation. See `MINT_TO_CHECKED.md`.

### CToken-Specific Features

**1. Compressible Top-Up Logic**
Automatically tops up CMint and CToken with rent lamports after minting to prevent accounts from becoming compressible.

**2. max_top_up Parameter**
10-byte instruction format adds `max_top_up` (u16) to limit combined top-up costs. Fails with `MaxTopUpExceeded` (18043) if exceeded.

### Unsupported SPL & Token-2022 Features

**1. No Multisig Support**
**2. No CPI Guard Extension Check**
**3. No Confidential Transfer Mint Extension Check**
