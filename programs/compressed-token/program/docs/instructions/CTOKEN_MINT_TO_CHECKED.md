## CToken MintToChecked

**discriminator:** 14
**enum:** `InstructionType::CTokenMintToChecked`
**path:** programs/compressed-token/program/src/ctoken_mint_to.rs

**description:**
Mints tokens from a decompressed CMint account to a destination CToken account with decimals validation, fully compatible with SPL Token MintToChecked semantics. Uses pinocchio-token-program to process the mint_to_checked operation which handles balance/supply updates, authority validation, frozen account checks, and decimals validation. After minting, automatically tops up compressible accounts with additional lamports if needed to prevent accounts from becoming compressible during normal operations. Both CMint and destination CToken can receive top-ups based on their current slot and account balance. Supports max_top_up parameter to limit rent top-up costs where 0 means no limit.

Account layouts:
- `CToken` defined in: program-libs/ctoken-interface/src/state/ctoken/ctoken_struct.rs
- `CompressedMint` (CMint) defined in: program-libs/ctoken-interface/src/state/mint/compressed_mint.rs
- `CompressionInfo` extension defined in: program-libs/compressible/src/compression_info.rs

**Instruction data:**
Path: programs/compressed-token/program/src/ctoken_mint_to.rs (lines 62-112, function `process_ctoken_mint_to_checked`)

Byte layout:
- Bytes 0-7: `amount` (u64, little-endian) - Number of tokens to mint
- Byte 8: `decimals` (u8) - Expected token decimals
- Bytes 9-10: `max_top_up` (u16, little-endian, optional) - Maximum lamports for top-ups combined, 0 = no limit

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
   - (signer, writable when top-ups needed)
   - Mint authority of the CMint account
   - Validated: must sign the transaction
   - Also serves as payer for rent top-ups if needed

**Instruction Logic and Checks:**

1. **Validate minimum accounts:**
   - Require at least 3 accounts (cmint, destination, authority)
   - Return NotEnoughAccountKeys if insufficient

2. **Parse instruction data:**
   - Require at least 9 bytes (amount + decimals)
   - Parse max_top_up from bytes 9-11 if present (11-byte format)
   - Default to 0 (no limit) if only 9 bytes provided (legacy format)
   - Return InvalidInstructionData if length is invalid (not 9 or 11 bytes)

3. **Process SPL mint_to_checked via pinocchio-token-program:**
   - Call `process_mint_to_checked` with first 9 bytes (amount + decimals)
   - Validates authority signature matches CMint mint authority
   - Validates decimals match CMint's decimals field
   - Checks destination CToken mint matches CMint
   - Checks destination CToken is not frozen
   - Increases destination CToken balance by amount
   - Increases CMint supply by amount
   - Errors are converted from pinocchio errors to ProgramError::Custom

4. **Calculate and execute top-up transfers:**
   - Calculate lamports needed for CMint based on compression state
   - Calculate lamports needed for CToken based on compression state
   - Validate total against max_top_up budget
   - Transfer lamports from authority to both accounts if needed

**Errors:**

- `ProgramError::NotEnoughAccountKeys` (error code: 11) - Less than 3 accounts provided
- `ProgramError::InvalidInstructionData` (error code: 3) - Instruction data length is not 9 or 11 bytes
- Pinocchio token errors (converted to ProgramError::Custom):
  - `TokenError::MintMismatch` (error code: 3) - CToken mint doesn't match CMint
  - `TokenError::OwnerMismatch` (error code: 4) - Authority doesn't match CMint mint_authority
  - `TokenError::MintDecimalsMismatch` (error code: 18) - Decimals don't match CMint's decimals
  - `TokenError::AccountFrozen` (error code: 17) - CToken account is frozen
- `CTokenError::CMintDeserializationFailed` (error code: 18047) - Failed to deserialize CMint account using zero-copy
- `CTokenError::InvalidAccountData` (error code: 18002) - Failed to deserialize CToken account or calculate top-up amount
- `CTokenError::SysvarAccessError` (error code: 18020) - Failed to get Clock or Rent sysvar for top-up calculation
- `CTokenError::MaxTopUpExceeded` (error code: 18043) - Total top-up amount (CMint + CToken) exceeds max_top_up limit

---

## Comparison with Token-2022

### Functional Parity

CToken MintToChecked maintains core compatibility with Token-2022's MintToChecked instruction:

- **Authority validation:** Both require mint authority signature and validate against the mint's configured mint_authority
- **Balance updates:** Both increase destination account balance and mint supply by the specified amount
- **Frozen account checks:** Both prevent minting to frozen accounts
- **Mint matching:** Both validate that destination account's mint field matches the mint account
- **Decimals validation:** Both validate that instruction decimals match mint decimals
- **Overflow protection:** Both check for arithmetic overflow when adding to balances and supply
- **Fixed supply enforcement:** Both fail if mint_authority is set to None (supply is fixed)

### CToken-Specific Features

1. **Compressible Top-Up Logic**: After minting, automatically replenishes lamports for compressible accounts
2. **max_top_up Parameter**: Limits combined lamports spent on CMint + CToken top-ups
3. **Authority Account Mutability**: Authority account must be writable when top-ups are needed

### Missing Features

1. **No Multisig Support**: Token-2022 supports multisig authorities via additional signer accounts
2. **No Extension Checks**: Token-2022's MintToChecked validates NonTransferable, PausableConfig, and ConfidentialMintBurn extensions

### Instruction Data Comparison

| Token-2022 MintToChecked | CToken MintToChecked |
|--------------------------|---------------------|
| 10 bytes (discriminator + amount + decimals) | 9 or 11 bytes (amount + decimals + optional max_top_up) |

### Account Layout Comparison

| Token-2022 MintToChecked | CToken MintToChecked |
|--------------------------|---------------------|
| [mint, destination, authority, ...signers] | [cmint, destination, authority] |
| 3+ accounts (for multisig) | Exactly 3 accounts |

### Security Properties

**Shared:**
- Authority signature validation before state changes
- Account ownership by token program validation
- Overflow prevention in balance/supply arithmetic
- Frozen account protection
- Decimals mismatch protection

**CToken-Specific:**
- Authority lamport drainage protection via max_top_up
- Top-up atomicity: if top-up fails, entire instruction fails
- Compressibility timing management
