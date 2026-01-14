## Claim

**discriminator:** 104
**enum:** `InstructionType::Claim`
**path:** programs/compressed-token/program/src/compressible/claim.rs

**description:**
1. Claims rent from compressible CToken and CMint solana accounts that have passed their rent expiration epochs
2. Supports both account types:
   - CToken (account_type = 2): decompressed token accounts, layout defined in program-libs/token-interface/src/state/ctoken/ctoken_struct.rs
   - CMint (account_type = 1): decompressed mint accounts, layout defined in program-libs/token-interface/src/state/mint/compressed_mint.rs
3. CompressionInfo storage differs by account type:
   - CToken: CompressionInfo is stored inside a Compressible extension (not embedded directly)
   - CMint: CompressionInfo is embedded directly in the mint struct at `compression` field
   - CompressionInfo type defined in program-libs/compressible/src/compression_info.rs
4. Processes multiple token accounts in a single instruction for efficiency
5. For each eligible compressible account:
   - Validates config_account_version matches CompressibleConfig version
   - Calculates claimable rent based on completed epochs since last claim
   - Updates the `last_claimed_slot` in the CompressionInfo
   - Updates the account's RentConfig from the CompressibleConfig (after claim calculation)
   - Transfers claimable lamports from token account to rent sponsor PDA
6. RentConfig is updated for ALL accounts that pass validation (even those without claimable rent)
7. CToken accounts must have Compressible extension; CMint accounts have CompressionInfo embedded directly
8. Only the compression authority (from CompressibleConfig) can execute claims
9. **Config validation:** Config must not be inactive (active or deprecated allowed)
10. Accounts that don't match compression_authority or rent_sponsor are skipped without error (returns None)
11. Accounts with mismatched config_account_version return error (CompressibleError::InvalidVersion)
12. Only completed epochs are claimed, partial epochs remain with the account
13. The instruction is designed to be called periodically by foresters

**Instruction data:**
- Empty (zero bytes required)
- Error if any instruction data is provided

**Accounts:**
1. rent_sponsor
   - (mutable)
   - The pool PDA that receives claimed rent
   - Must match the rent_sponsor in CompressibleConfig

2. compression_authority
   - (signer)
   - The authority authorized to claim rent
   - Must match compression_authority in CompressibleConfig
   - Typically a forester or system authority

3. compressible_config
   - (non-mutable)
   - CompressibleConfig account containing rent parameters
   - Owner must be Registry program (Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX)
   - Must not be in inactive state

4. accounts (remaining accounts)
   - (mutable, variable number)
   - CToken or CMint accounts to claim rent from
   - Account type determined by size: exactly 165 bytes = CToken, otherwise read byte 165 (1 = CMint, 2 = CToken)
   - Each account is processed independently
   - Accounts with wrong compression_authority or rent_sponsor are skipped without error (returns None)
   - Accounts with wrong owner, invalid size, or invalid type discriminator return error

**Instruction Logic and Checks:**

1. **Validate instruction data:**
   - Verify instruction data is empty
   - Error if any instruction data is provided

2. **Validate fixed accounts:**
   - Verify compression_authority is a signer
   - Verify compressible_config is owned by Registry program
   - Deserialize and validate CompressibleConfig:
     - Check config is not inactive (validate_not_inactive)
     - Verify compression_authority matches config
     - Verify rent_sponsor matches config

3. **Get current slot:**
   - Fetch from Clock sysvar for epoch calculation

4. **Process each account:**
   For each account in remaining accounts:

   a. **Verify account ownership:**
      - Account must be owned by the compressed token program
      - Uses `check_owner` with CTOKEN program ID

   b. **Determine account type:**
      - If account size < 165 bytes: invalid, return error
      - If account size == 165 bytes: CToken (legacy size without extensions)
      - If account size > 165 bytes: read byte 165 for discriminator (1 = CMint, 2 = CToken)

   c. **Parse account data:**
      - Borrow mutable data
      - Deserialize as CToken or CMint based on account type with zero-copy
      - For CToken: uses `CToken::zero_copy_at_mut_checked()` then `get_compressible_extension_mut()`
      - For CMint: uses `CompressedMint::zero_copy_at_mut_checked()` then accesses `base.compression`

   d. **Call claim_and_update (in CompressionInfo):**
      - Validate compression_authority matches (returns None if mismatch, skips account)
      - Validate rent_sponsor matches (returns None if mismatch, skips account)
      - Verify `config_account_version` matches CompressibleConfig version
        - Returns `CompressibleError::InvalidVersion` error if versions don't match
      - Call internal `claim()` method which:
        - Calculates claimable lamports based on completed epochs
        - Updates `last_claimed_slot` if there's claimable rent
        - Returns claimed amount or None if nothing to claim
      - Always update `rent_config` from CompressibleConfig (even if claim returned None)

   e. **Transfer lamports:**
      - If claim amount > 0, transfer from account to rent_sponsor
      - Uses `transfer_lamports` helper function

5. **Complete successfully:**
   - All accounts processed
   - Accounts with mismatched compression_authority/rent_sponsor are skipped (no error)

**Errors:**

- `ProgramError::InvalidInstructionData` (error code: 3) - Instruction data is not empty
- `ProgramError::InvalidAccountData` (error code: 4) - CompressibleConfig/CToken/CMint deserialization fails, account size < 165 bytes, or account type discriminator invalid
- `ErrorCode::InvalidCompressAuthority` - compression_authority doesn't match CompressibleConfig (fixed account validation)
- `ErrorCode::InvalidRentSponsor` - rent_sponsor doesn't match CompressibleConfig (fixed account validation)
- `CompressibleError::InvalidVersion` (error code: 19003) - Account's config_account_version doesn't match CompressibleConfig version (per-account validation)
- `CTokenError::MissingCompressibleExtension` (error code: 18056) - CToken account lacks required Compressible extension
- `AccountError::NotEnoughAccountKeys` (error code: 20014) - Missing required fixed accounts (rent_sponsor, compression_authority, config)
- `AccountError::InvalidSigner` (error code: 20009) - compression_authority is not a signer
- `AccountError::AccountNotMutable` (error code: 20002) - rent_sponsor is not mutable
- `AccountError::AccountOwnedByWrongProgram` (error code: 20001) - Token/Mint account not owned by compressed token program
- `CompressibleError::InvalidState` (error code: 19002) - CompressibleConfig is in inactive state

**Note on error vs skip behavior:**
- Fixed account validation errors (compression_authority, rent_sponsor, config) cause instruction failure
- Per-account compression_authority/rent_sponsor mismatch causes that account to be skipped (returns None)
- Per-account config version mismatch causes instruction failure with InvalidVersion error
