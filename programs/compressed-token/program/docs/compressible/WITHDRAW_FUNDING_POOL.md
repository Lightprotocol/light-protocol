## Withdraw Funding Pool

**discriminator:** 105
**enum:** `InstructionType::WithdrawFundingPool`
**path:** programs/compressed-token/program/src/compressible/withdraw_funding_pool.rs

**description:**
1. Withdraws lamports from the rent_sponsor PDA pool to a specified destination account
2. The rent_sponsor PDA holds funds collected from rent claims and compression incentives
3. Only the compression_authority from CompressibleConfig can execute withdrawals
4. **Config validation:** Config must not be inactive (active or deprecated allowed)
5. The rent_sponsor PDA is derived from ["rent_sponsor", version_bytes, bump] where version is a u16 from CompressibleConfig serialized as little-endian bytes
6. Enables protocol operators to manage collected rent and redirect funds for operational needs
7. The instruction validates rent_sponsor and compression_authority match the config

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
   - Required for pinocchio_system Transfer instruction

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
   - Parse all required accounts with correct mutability using AccountIterator
   - Verify compression_authority is signer
   - Parse and validate CompressibleConfig:
     - Check owner is Registry program
     - Validate discriminator and deserialize using bytemuck
     - Check config is not inactive (validate_not_inactive)
   - Verify compression_authority matches config
   - Verify rent_sponsor matches config
   - Extract rent_sponsor_bump and version (u16 as little-endian bytes) for PDA derivation

3. **Verify sufficient funds:**
   - Get current pool balance from rent_sponsor.lamports()
   - Check pool_lamports >= requested amount
   - Error if insufficient funds

4. **Execute transfer:**
   - Create pinocchio_system Transfer struct from rent_sponsor to destination
   - Prepare PDA signer seeds: [b"rent_sponsor", version_bytes (2 bytes), bump (1 byte)]
   - Invoke system program with PDA as signer using invoke_signed
   - Transfer specified amount to destination

**Errors:**

- `ProgramError::InvalidInstructionData` (error code: 3) - Instruction data length < 8 bytes or cannot parse amount from bytes
- `ProgramError::InvalidSeeds` (error code: 14) - compression_authority or rent_sponsor doesn't match CompressibleConfig
- `ProgramError::InvalidAccountData` (error code: 4) - CompressibleConfig deserialization fails or invalid discriminator
- `ProgramError::InsufficientFunds` (error code: 6) - Pool balance less than requested withdrawal amount (available balance shown in error message)
- `AccountError::NotEnoughAccountKeys` (error code: 20014) - Missing required accounts
- `AccountError::InvalidSigner` (error code: 20009) - compression_authority is not a signer
- `AccountError::AccountNotMutable` (error code: 20002) - rent_sponsor or destination is not mutable
- `CompressibleError::InvalidState` (error code: 19002) - CompressibleConfig is in inactive state