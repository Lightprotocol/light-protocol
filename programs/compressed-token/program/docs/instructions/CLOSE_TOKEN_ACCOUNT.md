## Close Token Account

**discriminator:** 9
**enum:** `CTokenInstruction::CloseTokenAccount`
**path:** programs/compressed-token/program/src/close_token_account/

**description:**
1. Closes decompressed ctoken solana accounts and distributes remaining lamports to destination account.
2. Account layout `CToken` is defined in path: program-libs/ctoken-interface/src/state/ctoken/ctoken_struct.rs
3. Supports both regular (non-compressible) and compressible token accounts (with compressible extension)
4. For compressible accounts (with compressible extension):
   - Rent exemption + unclaimed rent lamports are returned to the rent_sponsor
   - Remaining user lamports are returned to the destination account
   - Only the owner or close_authority (if set) can close using this instruction (balance must be zero)
   - **Note:** To compress and close with non-zero balance, use CompressAndClose mode in Transfer2 (compression_authority only)
   - **Note:** It is impossible to set a close authority.
5. For non-compressible accounts:
   - All lamports are transferred to the destination account
   - Only the owner or close_authority (if set) can close the account
6. After lamport distribution, the account is zeroed and resized to 0 bytes to prevent revival attacks

**Instruction data:**
- No instruction data required (empty)
- The instruction only reads the discriminator byte

**Accounts:**
1. token_account
   - (mutable)
   - The ctoken account being closed
   - Must be owned by the ctoken program
   - Must be initialized (not frozen or uninitialized)
   - Must have zero token balance
   - Data will be zeroed and account resized to 0

2. destination
   - (mutable)
   - Receives remaining user funds (non-rent lamports) for all account types
   - Cannot be the same as token_account

3. authority
   - (signer)
   - Must be the account's close_authority (if set) or owner (if close_authority is None)
   - Follows SPL Token behavior: close_authority takes precedence over owner
   - For compressible accounts: only owner/close_authority can close (compression_authority uses Transfer2 CompressAndClose instead)

4. rent_sponsor (optional, required for compressible accounts)
   - (mutable)
   - Receives rent exemption + unclaimed rent for compressible accounts
   - Must match the rent_sponsor field in the compressible extension
   - Not required for non-compressible accounts (only 3 accounts needed)

**Instruction Logic and Checks:**

1. **Parse and validate accounts** (`validate_and_parse` in `accounts.rs`):
   - Extract token_account (index 0), destination (index 1), authority (index 2)
   - Extract rent_sponsor (index 3) if accounts.len() >= 4 (required for compressible accounts)
   - Verify token_account is mutable via `check_mut`
   - Verify token_account is owned by ctoken program via `check_owner`
   - Verify destination is mutable via `check_mut`
   - Verify authority is a signer via `check_signer`
   - If rent_sponsor provided: verify rent_sponsor is mutable via `check_mut`

2. **Deserialize and validate token account** (`process_close_token_account` in `processor.rs`):
   - Borrow token account data mutably
   - Parse as `CToken` using `zero_copy_at_mut_checked` (validates initialized state and account type)
   - Call `validate_token_account<false>` (CHECK_RENT_AUTH=false for regular close)

3. **Validate closure requirements** (`validate_token_account<COMPRESS_AND_CLOSE: bool>`):
   3.1. **Basic validation**:
      - Verify token_account.key() != destination.key() (prevents self-transfer)

   3.2. **Balance check** (only when COMPRESS_AND_CLOSE=false):
      - Convert ctoken.amount from U64 to u64
      - Verify amount == 0 (non-zero returns `ErrorCode::NonNativeHasBalance`)

   3.3. **Compressible extension check**:
      - If account has extensions vector with `ZExtensionStructMut::Compressible`:
        - Get rent_sponsor from accounts (returns error if missing)
        - Verify compressible_ext.rent_sponsor == rent_sponsor.key()
        - Fall through to close_authority/owner check (compression_authority cannot use this instruction)

   3.4. **Account state check**:
      - Check account state field equals AccountState::Initialized (value 1):
        - If state == AccountState::Frozen (value 2): return `ErrorCode::AccountFrozen`
        - If state is any other value: return `ProgramError::UninitializedAccount`

   3.5. **Authority validation**:
      - Check close_authority field (SPL Token compatible behavior):
        - If close_authority is Some: verify authority.key() == close_authority (returns `ErrorCode::OwnerMismatch` if not)
        - If close_authority is None: verify authority.key() == ctoken.owner (returns `ErrorCode::OwnerMismatch` if not)
      - **Note:** For CompressAndClose mode in Transfer2, compression_authority validation is done separately (close_authority check does not apply)

4. **Distribute lamports** (`distribute_lamports` in `processor.rs`):
   4.1. **Setup**:
      - Get token_account.lamports() amount
      - Re-verify authority is signer via `check_signer`

   4.2. **Check for compressible extension**:
      - Borrow token account data (read-only this time)
      - Parse as CToken using `zero_copy_at_checked`
      - Look for `ZExtensionStruct::Compressible` in extensions

   4.3. **For compressible accounts** (if extension found):
      - Get current_slot from Clock::get() sysvar
      - Calculate base_lamports using `get_rent_exemption_lamports(account.data_len)`
      - Create `AccountRentState` with:
        - num_bytes, current_slot, current_lamports, last_claimed_slot
      - Call `calculate_close_distribution` with:
        - rent_config, base_lamports
      - Returns `CloseDistribution { to_rent_sponsor, to_user }`
      - Get rent_sponsor account from accounts (error if missing)
      - For regular close (owner/close_authority):
        - Transfer to_rent_sponsor lamports to rent_sponsor via `transfer_lamports` (if > 0)
        - Transfer to_user lamports to destination via `transfer_lamports` (if > 0)
      - For CompressAndClose (compression_authority in Transfer2):
        - Extract compression_cost from rent_sponsor portion as forester reward
        - Add to_user to rent_sponsor portion (unused funds go to rent_sponsor)
        - Transfer adjusted lamports to rent_sponsor and compression_cost to destination (forester)
      - Return early (skip non-compressible path)

   4.4. **For non-compressible accounts**:
      - Transfer all token_account.lamports to destination via `transfer_lamports`

5. **Finalize account closure** (`finalize_account_closure`):
   5.1. Zero the owner field:
      - Use unsafe block to call `token_account.assign(&[0u8; 32])`
      - Sets owner to system program (0x00000000...)

   5.2. Resize account to prevent revival:
      - Call `token_account.resize(0)`
      - Deallocates all account data
      - Maps resize error to ProgramError::Custom if fails

**Errors:**
- `ProgramError::InvalidAccountData` (error code: 4) - token_account == destination, rent_sponsor doesn't match extension, compression_authority mismatch, or account not compressible
- `ProgramError::NotEnoughAccountKeys` (error code: 11) - Missing rent_sponsor account for compressible accounts
- `AccountError::InvalidSigner` (error code: 12015) - Authority is not a signer
- `AccountError::AccountNotMutable` (error code: 12008) - token_account, destination, or rent_sponsor is not mutable
- `AccountError::AccountOwnedByWrongProgram` (error code: 12007) - token_account is not owned by ctoken program
- `AccountError::NotEnoughAccountKeys` (error code: 12020) - Not enough accounts provided
- `ErrorCode::AccountFrozen` (error code: 6076) - Account state is Frozen
- `ProgramError::UninitializedAccount` (error code: 10) - Account state is Uninitialized or invalid
- `ErrorCode::NonNativeHasBalance` (error code: 6074) - Account has non-zero token balance
- `ErrorCode::OwnerMismatch` (error code: 6075) - Authority doesn't match owner or close_authority
- `ProgramError::InsufficientFunds` (error code: 6) - Insufficient funds for lamport transfer during rent calculation

**Edge Cases and Considerations:**
- Only the close_authority (if set) or owner (if close_authority is None) can use this instruction (CloseTokenAccount)
- This matches SPL Token behavior where close_authority takes precedence over owner
- **Note:** SetAuthority instruction to set close_authority is currently unimplemented; close_authority is always None on newly created accounts
- For compression_authority to close accounts, use CompressAndClose mode in Transfer2
- Compressible accounts require 4 accounts, non-compressible require only 3
- Balance must be zero for this instruction (use Transfer2 CompressAndClose to compress non-zero balances)
- The instruction handles accounts with no extensions gracefully (non-compressible path)
- Zero-lamport accounts are handled without attempting transfers
- Separation of rent_sponsor from destination allows users to specify where their funds go while ensuring rent goes to the protocol
