# Add Token Pool

**discriminator:** `[114, 143, 210, 73, 96, 115, 1, 228]` program-libs/token-interface/src/discriminator.rs

**enum:** Not applicable - this is an Anchor instruction, not part of the custom `InstructionType` enum

**path:**
- Handler: `programs/compressed-token/anchor/src/lib.rs:68-95`
- Accounts struct: `programs/compressed-token/anchor/src/instructions/create_token_pool.rs:171-201`

**description:**
Token pool pda is renamed to spl interface pda in the light-token-sdk.
1. Creates additional token pools for a mint (indexes 1-4) after the initial pool (index 0) exists
2. Requires the previous pool (index-1) to exist, enforcing sequential pool creation. This ensures mint extensions were already validated during `create_token_pool` for pool index 0
3. Maximum 5 pools per mint (NUM_MAX_POOL_ACCOUNTS = 5, defined in programs/compressed-token/anchor/src/constants.rs)
4. Multiple pools enable scaling for high-volume mints by distributing token storage across accounts
5. For mints with restricted extensions (Pausable, PermanentDelegate, TransferFeeConfig, TransferHook, DefaultAccountState), uses a separate PDA derivation path with "restricted" seed to prevent accidental compression via legacy anchor instructions

**Instruction data:**
- `token_pool_index`: u8 - Pool index to create (valid values: 1-4)

**Accounts:**
1. fee_payer
   - (signer, mutable)
   - Pays for account creation (rent-exempt deposit + transaction fees)
2. token_pool_pda
   - (mutable)
   - New token pool account being created
   - PDA derivation (regular mints): seeds=[b"pool", mint_pubkey, token_pool_index], program=light_compressed_token
   - PDA derivation (restricted mints): seeds=[b"pool", mint_pubkey, b"restricted", token_pool_index], program=light_compressed_token
   - Owner set to token_program
3. existing_token_pool_pda
   - Existing token pool at index (token_pool_index - 1)
   - Must be a valid SPL/Token-2022 TokenAccount
   - Validates sequential pool creation
4. system_program
   - System program for account allocation
5. mint
   - SPL Token or Token-2022 mint account
   - Validated: must be owned by token_program
6. token_program
   - Token program interface (SPL Token or Token-2022)
7. cpi_authority_pda
   - CPI authority PDA
   - PDA derivation: seeds=[b"light_cpi_authority"], program=light_compressed_token
   - Becomes the owner/authority of the new token pool account

**Instruction Logic and Checks:**
1. Validate token_pool_index < NUM_MAX_POOL_ACCOUNTS (5)
   - Error: InvalidTokenPoolBump if index >= 5
2. Determine if mint has restricted extensions via `restricted_seed()` (programs/compressed-token/anchor/src/instructions/create_token_pool.rs:21-39)
   - Checks for: Pausable, PermanentDelegate, TransferFeeConfig, TransferHook, DefaultAccountState extensions
3. Validate previous pool exists via `is_valid_spl_interface_pda()` (program-libs/token-interface/src/pool_derivation.rs:95-148)
   - Uses `token_pool_index.saturating_sub(1)` as the previous index
   - Verifies existing_token_pool_pda matches PDA derivation with (token_pool_index - 1)
   - Uses the same restricted/regular derivation path as the new pool
   - Error: InvalidTokenPoolPda if previous pool doesn't exist or has wrong derivation
4. Initialize token account via CPI to `spl_token_2022::instruction::initialize_account3` (same as create_token_pool)

**CPIs:**
- `spl_token_2022::instruction::initialize_account3`
  - Target program: token_program (SPL Token or Token-2022)
  - Accounts: [token_pool_pda, mint, cpi_authority_pda, token_program]
  - Purpose: Initializes the new token pool as a valid SPL token account with cpi_authority_pda as owner

**Errors:**
- `InvalidTokenPoolBump` (6029) - token_pool_index >= NUM_MAX_POOL_ACCOUNTS (max 5 pools reached)
- `InvalidTokenPoolPda` (6023) - Previous pool at (index-1) doesn't exist or has invalid PDA derivation
- `InvalidMint` (6126) - Mint account fails to deserialize (from `get_token_account_space`)
- Anchor `ConstraintSeeds` - PDA derivation failed
- Anchor `AccountAlreadyInUse` - Token pool already exists at this index
- `InsufficientFunds` - Fee payer has insufficient lamports
