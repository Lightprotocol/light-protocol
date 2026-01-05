## Claim

**discriminator:** 104
**enum:** `InstructionType::Claim`
**path:** programs/compressed-token/program/src/claim.rs

**description:**
1. Claims rent from compressible CToken and CMint solana accounts that have passed their rent expiration epochs
2. Supports both account types:
   - CToken (account_type = 2): decompressed token accounts, layout defined in program-libs/ctoken-interface/src/state/ctoken/ctoken_struct.rs
   - CMint (account_type = 1): decompressed mint accounts, layout defined in program-libs/ctoken-interface/src/state/mint/compressed_mint.rs
3. CompressionInfo is embedded directly in both account types (not as an extension), defined in program-libs/compressible/src/compression_info.rs
4. Processes multiple token accounts in a single instruction for efficiency
5. For each eligible compressible account:
   - Updates the account's RentConfig from the CompressibleConfig
   - Updates the config_account_version to match current config version
   - Calculates claimable rent based on completed epochs since last claim
   - Updates the `last_claimed_slot` in the compressible extension
   - Transfers claimable lamports from token account to rent sponsor PDA
6. RentConfig is updated for ALL accounts with compressible extension (even those without claimable rent)
7. Only accounts with compressible extension can be claimed from
8. Only the compression authority (from CompressibleConfig) can execute claims
9. **Config validation:** Config must not be inactive (active or deprecated allowed)
10. Accounts that don't meet claim criteria are skipped without error
11. Only completed epochs are claimed, partial epochs remain with the account
12. The instruction is designed to be called periodically by foresters

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
   - Account type determined by byte 165 (1 = CMint, 2 = CToken) or size (165 bytes = CToken)
   - Each account is processed independently
   - Invalid accounts (wrong authority/recipient/type) are skipped without error

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

   a. **Determine account type:**
      - If account size < 165 bytes: invalid, skip
      - If account size == 165 bytes: CToken (legacy)
      - If account size > 165 bytes: read byte 165 for discriminator (1 = CMint, 2 = CToken)

   b. **Parse account data:**
      - Borrow mutable data
      - Deserialize as CToken or CMint based on account type with zero-copy

   c. **Validate compression info:**
      - Access embedded CompressionInfo from account
      - Validate compression_authority matches
      - Validate rent_sponsor matches

   d. **Validate version:**
      - Verify `compression.config_account_version` matches CompressibleConfig version
      - Error with `CompressibleError::InvalidVersion` if versions don't match (prevents cross-version claims)

   e. **Calculate and claim rent:**
      - Get account size and current lamports
      - Calculate rent exemption for account size
      - Call `compression.claim()` which:
        - Determines completed epochs since last claim using CURRENT RentConfig
        - Calculates claimable lamports
        - Updates last_claimed_slot if there's claimable rent
      - Returns None if no rent to claim (account not yet compressible)
      - After claim calculation, always update `compression.rent_config` from CompressibleConfig for future operations

   f. **Transfer lamports:**
      - If claim amount > 0, transfer from account to rent_sponsor
      - Update both account balances

5. **Complete successfully:**
   - All valid accounts processed
   - Invalid accounts silently skipped

**Errors:**

- `ProgramError::InvalidInstructionData` (error code: 3) - Instruction data is not empty
- `ProgramError::InvalidAccountData` (error code: 4) - CompressibleConfig/CToken deserialization fails, account type discriminator invalid, or claim calculation fails
- `ErrorCode::InvalidCompressAuthority` - compression_authority doesn't match CompressibleConfig
- `ErrorCode::InvalidRentSponsor` - rent_sponsor doesn't match CompressibleConfig
- `CompressibleError::InvalidVersion` (error code: 19003) - Account's config_account_version doesn't match CompressibleConfig version
- `CTokenError::MissingCompressibleExtension` (error code: 18056) - CToken account lacks required Compressible extension
- `AccountError::NotEnoughAccountKeys` (error code: 20014) - Missing required accounts
- `AccountError::InvalidSigner` (error code: 20009) - compression_authority is not a signer
- `AccountError::AccountNotMutable` (error code: 20002) - rent_sponsor is not mutable
- `AccountError::AccountOwnedByWrongProgram` (error code: 20001) - Token account not owned by compressed token program
- `CompressibleError::InvalidState` (error code: 19002) - CompressibleConfig is in inactive state
