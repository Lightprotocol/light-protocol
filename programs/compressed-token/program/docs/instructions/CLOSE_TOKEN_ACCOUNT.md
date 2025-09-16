## Close Token Account

**discriminator:** 9
**enum:** `CTokenInstruction::CloseTokenAccount`
**path:** programs/compressed-token/program/src/close_token_account/

**description:**
1. Closes decompressed ctoken solana accounts and distributes remaining lamports to destination account.
2. Account layout `CompressedToken` is defined in path: program-libs/ctoken-types/src/state/solana_ctoken.rs
3. Supports both regular (non-compressible) and compressible token accounts (with compressible extension)
4. For compressible accounts (with compressible extension):
   - Rent exemption is returned to the rent recipient (destination account)
   - Write top-up lamports are returned to the authority (original fee payer)
   - Authority can be either the owner OR the rent authority (if account is compressible)
5. For non-compressible accounts:
   - All lamports are transferred to the destination account
   - Only the owner can close the account
6. After lamport distribution, the account is zeroed and resized to 0 bytes to prevent revival attacks

**Instruction data:**
- No instruction data required (empty)
- The instruction only reads the discriminator byte

**Accounts:**
1. token_account
   - (mutable)
   - The ctoken account being closed
   - Must be initialized (not frozen or uninitialized)
   - Must have zero token balance
   - Data will be zeroed and account resized to 0

2. destination
   - (mutable)
   - Receives rent exemption for compressible accounts or all lamports for non-compressible
   - Cannot be the same as token_account
   - For compressible accounts, must match the rent_recipient in the extension

3. authority
   - (signer)
   - Either the account owner OR rent authority (for compressible accounts)
   - For compressible accounts closed by rent authority:
     - Account must be compressible (past rent expiry)
     - Authority must match rent_authority in extension
   - Receives write top-up lamports if any (compressible accounts only)

**Instruction Logic and Checks:**

1. **Parse and validate accounts** (`validate_and_parse` in `accounts.rs`):
   - Extract token_account (index 0), destination (index 1), authority (index 2)
   - Verify token_account is mutable via `check_mut`
   - Verify destination is mutable via `check_mut`
   - Verify authority is a signer via `check_signer`

2. **Deserialize and validate token account** (`process_close_token_account` in `processor.rs`):
   - Borrow token account data mutably
   - Parse as `CompressedToken` using `zero_copy_at_mut` (zero-copy deserialization)
   - Call `validate_token_account<false>` (CHECK_RENT_AUTH=false for regular close)

3. **Validate closure requirements** (`validate_token_account<CHECK_RENT_AUTH: bool>`):
   3.1. **Basic validation**:
      - Verify token_account.key() != destination.key() (prevents self-transfer)
      - Check account state field equals AccountState::Initialized (value 1):
        - If state == AccountState::Frozen (value 2): return `ErrorCode::AccountFrozen`
        - If state is any other value: return `ProgramError::UninitializedAccount`

   3.2. **Balance check** (only when CHECK_RENT_AUTH=false):
      - Convert compressed_token.amount from U64 to u64
      - Verify amount == 0 (non-zero returns `ErrorCode::NonNativeHasBalance`)

   3.3. **Authority validation**:
      - Check if compressed_token.owner == authority.key() (store as `owner_matches`)
      - If account has extensions vector:
        3.3.1. Iterate through extensions looking for `ZExtensionStructMut::Compressible`
        3.3.2. If compressible extension found:
          - Verify compressible_ext.rent_recipient == destination.key()
          - If not owner_matches and CHECK_RENT_AUTH=true:
            - Verify compressible_ext.rent_authority == authority.key()
            - Get current slot from Clock sysvar
            - Call `compressible_ext.is_compressible(data_len, current_slot, lamports)`
            - If not compressible: return error
            - Return Ok((true, compress_to_pubkey_flag))
      - If owner doesn't match and no valid rent authority: return `ErrorCode::OwnerMismatch`

4. **Distribute lamports** (`close_token_account_inner`):
   4.1. **Setup**:
      - Get token_account.lamports() amount
      - Re-verify authority is signer via `check_signer`

   4.2. **Check for compressible extension**:
      - Borrow token account data (read-only this time)
      - Parse as CompressedToken using `zero_copy_at`
      - Look for `ZExtensionStruct::Compressible` in extensions

   4.3. **For compressible accounts** (if extension found):
      - Get current_slot from Clock::get() sysvar
      - Calculate base_lamports using `get_rent_exemption_lamports(account.data_len)`
      - Extract from compressible_ext.rent_config:
        - min_rent (u16 -> u64)
        - rent_per_byte (u8 -> u64)
        - full_compression_incentive (u16 -> u64)
      - Call `calculate_close_lamports` with:
        - data_len, current_slot, total_lamports
        - last_claimed_slot, base_lamports
        - min_rent, rent_per_byte, full_compression_incentive
      - Returns (lamports_to_destination, lamports_to_authority)
      - Special case: if authority.key() == rent_authority:
        - Add lamports_to_authority to lamports_to_destination
        - Set lamports_to_authority = 0
      - Transfer lamports_to_destination to destination via `transfer_lamports`
      - Transfer lamports_to_authority to authority via `transfer_lamports` (if > 0)
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
1. `ProgramError::InvalidAccountData`
   - token_account == destination
   - rent_recipient doesn't match destination (compressible)
   - rent_authority doesn't match authority when closing as rent authority
   - Account not compressible when rent authority tries to close

2. `ErrorCode::AccountFrozen`
   - Account state is Frozen

3. `ProgramError::UninitializedAccount`
   - Account state is Uninitialized or invalid

4. `ErrorCode::NonNativeHasBalance`
   - Account has non-zero token balance

5. `ErrorCode::OwnerMismatch`
   - Authority doesn't match owner and isn't valid rent authority

6. `ProgramError::MissingRequiredSignature`
   - Authority is not a signer

7. `ProgramError::Custom`
   - Failed to get clock sysvar
   - Lamport transfer failures
   - Account resize failures

**Edge Cases and Considerations:**
- When rent authority closes an account, they don't receive the write top-up; it all goes to destination
- The timing check for compressibility uses current slot vs last_claimed_slot
- The instruction handles accounts with no extensions gracefully (non-compressible path)
- Zero-lamport accounts are handled without attempting transfers
