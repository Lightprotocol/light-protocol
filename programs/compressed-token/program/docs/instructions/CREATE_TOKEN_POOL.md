# Create Token Pool

**path:** programs/compressed-token/anchor/src/lib.rs:49-62

**description:**
Token pool pda is renamed to spl interface pda in the light-token-sdk.
1. Creates a token pool PDA for a given SPL or Token-2022 mint
2. Token pools store underlying SPL/T22 tokens when users compress them into compressed tokens or convert them into ctokens. When tokens are compressed, they are transferred to the pool; when decompressed, tokens are transferred back from the pool to the user
3. Each mint can have up to 5 token pools (this instruction creates the first pool at index 0)
4. Validates mint extensions against the allowed list (16 supported Token-2022 extensions)
5. Initializes the token account via CPI to the token program with `cpi_authority_pda` as the account owner/authority

**Instruction data:**
- No instruction parameters (all configuration derived from accounts)

**Accounts:**
1. fee_payer
   - (signer, mutable)
   - Pays for account creation (rent-exempt deposit + transaction fees)
2. token_pool_pda
   - (mutable)
   - New token pool account being created
   - PDA derivation: seeds=[b"pool", mint_pubkey], program=light_compressed_token
   - Owner set to token_program
3. system_program
   - System program for account allocation
4. mint
   - SPL Token or Token-2022 mint account
   - Validated: must be owned by token_program
   - Extensions are checked against ALLOWED_EXTENSION_TYPES
5. token_program
   - Token program interface (SPL Token or Token-2022)
6. cpi_authority_pda
   - CPI authority PDA
   - PDA derivation: seeds=[b"light_cpi_authority"], program=light_compressed_token
   - Becomes the owner/authority of the token pool account

**Instruction Logic and Checks:**
1. Validate mint extensions via `assert_mint_extensions()` (programs/compressed-token/anchor/src/instructions/create_token_pool.rs:106-142)
   - All extensions must be in ALLOWED_EXTENSION_TYPES (program-libs/ctoken-interface/src/token_2022_extensions.rs:23-43)
   - Allowed extensions (16 types): MetadataPointer, TokenMetadata, InterestBearingConfig, GroupPointer, GroupMemberPointer, TokenGroup, TokenGroupMember, MintCloseAuthority, TransferFeeConfig, DefaultAccountState, PermanentDelegate, TransferHook, Pausable, ConfidentialTransferMint, ConfidentialTransferFeeConfig, ConfidentialMintBurn
   - **Restricted extensions (require specific configuration):**
     - `TransferFeeConfig` - fees must be zero (both `older_transfer_fee` and `newer_transfer_fee` must have `transfer_fee_basis_points == 0` and `maximum_fee == 0`)
     - `TransferHook` - program_id must be nil (no active transfer hook program)
     - `PermanentDelegate` - allowed, but marks token for compression_only mode at runtime
     - `Pausable` - allowed, but pause state checked at transfer time from SPL mint
2. Anchor allocates account space based on mint extensions via `get_token_account_space()` (programs/compressed-token/anchor/src/instructions/create_token_pool.rs:51-61)
3. Initialize token account via CPI to `spl_token_2022::instruction::initialize_account3` (programs/compressed-token/anchor/src/instructions/create_token_pool.rs:64-86)

**CPIs:**
- `spl_token_2022::instruction::initialize_account3`
  - Target program: token_program (SPL Token or Token-2022)
  - Accounts: [token_pool_pda, mint, cpi_authority_pda, token_program]
  - Purpose: Initializes the token pool as a valid SPL token account with cpi_authority_pda as owner

**Errors:**
- `InvalidMint` (6126) - Mint account fails to deserialize as PodStateWithExtensions<PodMint>
- `MintWithInvalidExtension` (6027) - Mint has an extension not in ALLOWED_EXTENSION_TYPES
- `NonZeroTransferFeeNotSupported` (6129) - Mint has TransferFeeConfig with non-zero transfer_fee_basis_points or maximum_fee
- `TransferHookNotSupported` (6130) - Mint has TransferHook extension with non-nil program_id
- Anchor `ConstraintSeeds` - PDA derivation failed (wrong mint key or bump)
- Anchor `AccountAlreadyInUse` - Token pool already exists for this mint
- `InsufficientFunds` - Fee payer has insufficient lamports for rent-exempt deposit
