## Withdraw Funding Pool

**discriminator:** 105
**enum:** `InstructionType::WithdrawFundingPool`
**path:** programs/compressed-token/program/src/withdraw_funding_pool.rs

**description:**
1. Withdraws lamports from the rent_sponsor PDA pool to a specified destination account
2. The rent_sponsor PDA holds funds collected from rent claims and compression incentives
3. Only the compression_authority from CompressibleConfig can execute withdrawals
4. **Config validation:** Config must not be inactive (active or deprecated allowed)
5. The rent_sponsor PDA is derived from ["rent_sponsor", version_bytes, bump] where version comes from CompressibleConfig
6. Enables protocol operators to manage collected rent and redirect funds for operational needs
7. The instruction validates PDA derivation matches the config's rent_sponsor

**Instruction data:**
- First 8 bytes: withdrawal amount (u64, little-endian)
- Amount must not exceed available pool balance

**Accounts:**
1. rent_sponsor
   - (mutable)
   - The pool PDA holding collected rent and compression incentives
   - Must match rent_sponsor in CompressibleConfig
   - Signs the system transfer via PDA seeds

2. compression_authority
   - (signer)
   - Authority authorized to withdraw from pool
   - Must match compression_authority in CompressibleConfig
   - Only this authority can withdraw funds

3. destination
   - (mutable)
   - Account to receive the withdrawn lamports
   - Can be any valid Solana account

4. system_program
   - (non-mutable)
   - System program for lamport transfer
   - Required for system_instruction::transfer

5. config
   - (non-mutable)
   - CompressibleConfig account containing pool configuration
   - Owner must be Registry program (Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX)
   - Must not be in inactive state
   - Used to validate authorities and PDA derivation

**Instruction Logic and Checks:**

1. **Parse instruction data:**
   - Extract amount from first 8 bytes as u64 little-endian
   - Error if instruction data length < 8 bytes

2. **Validate and parse accounts:**
   - Parse all required accounts with correct mutability
   - Verify compression_authority is signer
   - Parse and validate CompressibleConfig:
     - Deserialize using parse_config_account helper
     - Check config is not inactive (validate_not_inactive)
   - Verify compression_authority matches config
   - Verify rent_sponsor matches config
   - Extract rent_sponsor_bump and version for PDA derivation

3. **Verify sufficient funds:**
   - Get current pool balance from rent_sponsor.lamports()
   - Check pool_lamports >= requested amount
   - Error if insufficient funds

4. **Execute transfer:**
   - Create system_instruction::transfer from rent_sponsor to destination
   - Prepare PDA signer seeds: ["rent_sponsor", version_bytes, bump]
   - Invoke system program with PDA as signer using invoke_signed
   - Transfer specified amount to destination

**Errors:**

- `ProgramError::InvalidInstructionData` (error code: 3) - Instruction data length < 8 bytes or cannot parse amount from bytes
- `ProgramError::InvalidSeeds` (error code: 14) - compression_authority or rent_sponsor doesn't match CompressibleConfig
- `ProgramError::InvalidAccountData` (error code: 4) - CompressibleConfig deserialization fails or invalid discriminator
- `ProgramError::InsufficientFunds` (error code: 6) - Pool balance less than requested withdrawal amount (available balance shown in error message)
- `AccountError::NotEnoughAccountKeys` (error code: 12020) - Missing required accounts
- `AccountError::InvalidSigner` (error code: 12015) - compression_authority is not a signer
- `AccountError::AccountNotMutable` (error code: 12008) - rent_sponsor or destination is not mutable
- `CompressibleError::InvalidState` (error code: 19002) - CompressibleConfig is in inactive state