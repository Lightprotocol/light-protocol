## Claim

**discriminator:** 104
**enum:** `InstructionType::Claim`
**path:** programs/compressed-token/program/src/claim/

**description:**
1. Claims rent from compressible ctoken solana accounts that have passed their rent expiration epochs
2. Account layout `CToken` is defined in path: program-libs/ctoken-types/src/state/ctoken/ctoken_struct.rs
3. Extension layout `CompressionInfo` is defined in path: program-libs/ctoken-types/src/state/extensions/compressible.rs
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
11. Multiple accounts can be claimed in a single transaction for efficiency
12. Only completed epochs are claimed, partial epochs remain with the account
13. The instruction is designed to be called periodically by foresters

**Instruction data:**
- Single byte: pool PDA bump
- Used to validate the rent_sponsor PDA derivation

**Accounts:**
1. rent_sponsor
   - (mutable)
   - The pool PDA that receives claimed rent
   - Must match the rent_sponsor in CompressibleConfig
   - Derivation validated using provided bump

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

4. token_accounts (remaining accounts)
   - (mutable, variable number)
   - CToken accounts to claim rent from
   - Each account is processed independently
   - Accounts without compressible extension are skipped
   - Invalid accounts (wrong authority/recipient) are skipped without error

**Instruction Logic and Checks:**

1. **Parse instruction data:**
   - Extract pool PDA bump from first byte
   - Error if instruction data is empty

2. **Validate fixed accounts:**
   - Verify compression_authority is a signer
   - Verify compressible_config is owned by Registry program
   - Deserialize and validate CompressibleConfig:
     - Check config is not inactive (validate_not_inactive)
     - Verify compression_authority matches config
     - Verify rent_sponsor matches config

3. **Get current slot:**
   - Fetch from Clock sysvar for epoch calculation

4. **Process each token account:**
   For each account in remaining accounts:

   a. **Parse account data:**
      - Borrow mutable data
      - Deserialize as CToken with zero-copy

   b. **Find and validate compressible extension:**
      - Search extensions for Compressible variant
      - Skip if no compressible extension found
      - Validate compression_authority matches
      - Validate rent_sponsor matches

   c. **Validate version:**
      - Verify `compressible_ext.config_account_version` matches CompressibleConfig version
      - Error if versions don't match (prevents cross-version claims)

   d. **Calculate and claim rent:**
      - Get account size and current lamports
      - Calculate rent exemption for account size
      - Call `compressible_ext.claim()` which:
        - Determines completed epochs since last claim using CURRENT RentConfig
        - Calculates claimable lamports
        - Updates last_claimed_slot if there's claimable rent
      - Returns None if no rent to claim (account not yet compressible)
      - After claim calculation, always update `compressible_ext.rent_config` from CompressibleConfig for future operations

   e. **Transfer lamports:**
      - If claim amount > 0, transfer from token account to rent_sponsor
      - Update both account balances

5. **Complete successfully:**
   - All valid accounts processed
   - Invalid accounts silently skipped

**Errors:**

- `ProgramError::InvalidInstructionData` (error code: 3) - Missing pool PDA bump in instruction data or instruction data is empty
- `ProgramError::InvalidSeeds` (error code: 14) - compression_authority or rent_sponsor doesn't match CompressibleConfig
- `ProgramError::InvalidAccountData` (error code: 4) - CompressibleConfig/CToken deserialization fails, config version mismatch, or claim calculation fails
- `AccountError::NotEnoughAccountKeys` (error code: 12020) - Missing required accounts
- `AccountError::InvalidSigner` (error code: 12015) - compression_authority is not a signer
- `AccountError::AccountNotMutable` (error code: 12008) - rent_sponsor is not mutable
- `CompressibleError::InvalidState` (error code: 19002) - CompressibleConfig is in inactive state
