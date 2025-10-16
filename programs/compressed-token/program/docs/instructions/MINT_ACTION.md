## MintAction

**discriminator:** 103
**enum:** `CTokenInstruction::MintAction`
**path:** programs/compressed-token/program/src/mint_action/

**description:**
Batch instruction for managing compressed mint accounts (cmints) and performing mint operations. A compressed mint account stores the mint's supply, decimals, authorities (mint/freeze), and optional TokenMetadata extension in compressed state. TokenMetadata is the only extension supported for compressed mints and provides fields for name, symbol, uri, update_authority, and additional key-value metadata.

This instruction supports 9 total actions - one creation action (controlled by `create_mint` flag) and 8 enum-based actions:

**Compressed mint creation (executed first when `create_mint=true`):**
1. **Create Compressed Mint** - Create a new compressed mint account with initial authorities and optional TokenMetadata extension

**Core mint operations (Action enum variants):**
2. `MintToCompressed` - Mint new compressed tokens to one or more compressed token accounts
3. `MintToCToken` - Mint new tokens to decompressed ctoken accounts (not SPL tokens)
4. `CreateSplMint` - Create an SPL Token 2022 mint for an existing compressed mint, enabling SPL interoperability

**Authority updates (Action enum variants):**
5. `UpdateMintAuthority` - Update or remove the mint authority
6. `UpdateFreezeAuthority` - Update or remove the freeze authority

**TokenMetadata extension operations (Action enum variants):**
7. `UpdateMetadataField` - Update name, symbol, uri, or additional_metadata fields in the TokenMetadata extension
8. `UpdateMetadataAuthority` - Update the metadata update authority in the TokenMetadata extension
9. `RemoveMetadataKey` - Remove a key-value pair from additional_metadata in the TokenMetadata extension

Key concepts integrated:
- **Compressed mint (cmint)**: Mint state stored in compressed account with deterministic address derived from associated SPL mint pubkey
- **SPL mint synchronization**: When SPL mint exists, supply is tracked in both compressed mint and SPL mint through token pool PDAs
- **Authority validation**: All actions require appropriate authority (mint/freeze/metadata) to be transaction signer
- **Batch processing**: Multiple actions execute sequentially with state updates persisted between actions

**Instruction data:**
1. instruction data is defined in path: program-libs/ctoken-types/src/instructions/mint_action/instruction_data.rs

   **Core fields:**
   - `create_mint`: bool - Whether creating new compressed mint (true) or updating existing (false)
   - `mint_bump`: u8 - PDA bump for SPL mint derivation (only used if create_mint=true)
   - `leaf_index`: u32 - Merkle tree leaf index of existing compressed mint (only used if create_mint=false)
   - `prove_by_index`: bool - Use proof-by-index for existing mint validation (only used if create_mint=false)
   - `root_index`: u16 - Root index for address proof (create) or validity proof (update)
   - `compressed_address`: [u8; 32] - Deterministic address derived from SPL mint pubkey
   - `token_pool_bump`: u8 - Token pool PDA bump (required for SPL mint operations)
   - `token_pool_index`: u8 - Token pool PDA index (required for SPL mint operations)
   - `actions`: Vec<Action> - Ordered list of actions to execute
   - `proof`: Option<CompressedProof> - ZK proof for compressed account validation (required unless prove_by_index=true)
   - `cpi_context`: Option<CpiContext> - For cross-program invocation support
   - `mint`: CompressedMintInstructionData - Full mint state including supply, decimals, metadata, authorities, and extensions

2. Action types (path: program-libs/ctoken-types/src/instructions/mint_action/):
   - `MintToCompressed(MintToCompressedAction)` - Mint tokens to compressed accounts (mint_to.rs)
   - `UpdateMintAuthority(UpdateAuthority)` - Update mint authority (update_mint.rs)
   - `UpdateFreezeAuthority(UpdateAuthority)` - Update freeze authority (update_mint.rs)
   - `CreateSplMint(CreateSplMintAction)` - Create SPL mint for cmint (create_spl_mint.rs)
   - `MintToCToken(MintToCTokenAction)` - Mint to ctoken accounts (mint_to_ctoken.rs)
   - `UpdateMetadataField(UpdateMetadataFieldAction)` - Update metadata field (update_metadata.rs)
   - `UpdateMetadataAuthority(UpdateMetadataAuthorityAction)` - Update metadata authority (update_metadata.rs)
   - `RemoveMetadataKey(RemoveMetadataKeyAction)` - Remove metadata key (update_metadata.rs)

**Accounts:**
1. light_system_program
   - non-mutable
   - Light Protocol system program for cpi to create or update the compressed mint account.

Optional accounts (based on configuration):
2. mint_signer
   - (signer) - required if create_mint=true or CreateSplMint action present
   - PDA seed for SPL mint creation (seeds from compressed mint randomness)

3. authority
   - (signer)
   - Must match current mint/freeze/metadata authority for respective actions

For execution (when not writing to CPI context):
4. mint
   - (mutable) - optional, required if spl_mint_initialized=true
   - SPL Token 2022 mint account for supply synchronization

5. token_pool_pda
   - (mutable) - optional, required if spl_mint_initialized=true
   - Token pool PDA that holds SPL tokens backing compressed supply
   - Derivation: [mint, token_pool_index] with token_pool_bump

6. token_program
   - non-mutable - optional, required if spl_mint_initialized=true
   - Must be SPL Token 2022 program (validated in accounts.rs:126)

7-12. Light system accounts (standard set):
   - fee_payer (signer, mutable)
   - cpi_authority_pda
   - registered_program_pda
   - account_compression_authority
   - account_compression_program
   - system_program

13. out_output_queue
   - (mutable)
   - Output queue for compressed mint account updates

14. address_merkle_tree OR in_merkle_tree
   - (mutable)
   - If create_mint=true: address_merkle_tree for new mint (must be CMINT_ADDRESS_TREE)
   - If create_mint=false: in_merkle_tree for existing mint validation

15. in_output_queue
   - (mutable) - optional, required if create_mint=false
   - Input queue for existing compressed mint

16. tokens_out_queue
   - (mutable) - optional, required for MintToCompressed actions
   - Output queue for newly minted compressed token accounts

For CPI context write (when write_to_cpi_context=true):
4-6. CPI context accounts only

Packed accounts (remaining accounts):
- Merkle tree and queue accounts for compressed storage
- Recipient ctoken accounts for MintToCToken action

**Instruction Logic and Checks:**

1. **Parse and validate instruction data:**
   - Deserialize `MintActionCompressedInstructionData` using zero-copy
   - Validate proof exists unless prove_by_index=true
   - Configure account requirements based on actions

2. **Validate and parse accounts:**
   - Check authority is signer
   - If SPL mint initialized: validate token pool PDA derivation
   - Validate mint account matches expected cmint pubkey
   - For create_mint: validate address_merkle_tree is CMINT_ADDRESS_TREE
   - Extract packed accounts for dynamic operations

3. **Process mint creation or input:**
   - If create_mint=true:
     - Derive SPL mint PDA from compressed address
     - Set create address in CPI instruction
   - If create_mint=false:
     - Hash existing compressed mint account
     - Set input with merkle context (tree, queue, leaf_index, proof)

4. **Process actions sequentially:**
   Each action validates authority and updates compressed mint state:

   **MintToCompressed:**
   - Validate: mint authority matches signer
   - Calculate: sum recipient amounts with overflow protection
   - Update: mint supply += sum_amounts
   - If SPL mint exists: mint equivalent tokens to pool via CPI
   - Create: compressed token accounts for each recipient

   **UpdateMintAuthority / UpdateFreezeAuthority:**
   - Validate: current authority matches signer
   - Update: set new authority (can be None to disable)

   **CreateSplMint:**
   - Validate: mint_signer is provided and signing
   - Create: SPL Token 2022 mint account via CPI
   - Create: Token pool PDA account
   - Initialize: mint with ctoken PDA as mint/freeze authority
   - Mint: existing supply to token pool

   **MintToCToken:**
   - Validate: mint authority matches signer
   - Calculate: sum recipient amounts
   - Update: mint supply += sum_amounts
   - If SPL mint exists: mint to pool, then transfer to recipients
   - If no SPL mint: directly update ctoken account balances

   **UpdateMetadataField:**
   - Validate: metadata authority matches signer (defaults to mint authority)
   - Find: TokenMetadata extension at specified index
   - Update: specified field (name/symbol/uri/additional_metadata)

   **UpdateMetadataAuthority:**
   - Validate: current metadata authority matches signer
   - Update: set new metadata update authority

   **RemoveMetadataKey:**
   - Validate: metadata authority matches signer
   - Find: key in additional_metadata
   - Remove: key-value pair from metadata

5. **Finalize output compressed mint:**
   - Hash updated mint state
   - Set output compressed account with new state root
   - Assign to appropriate merkle tree

6. **Execute CPI to light-system-program:**
   - Build CPI accounts array
   - Include tree pubkeys for merkle operations
   - Execute with or without CPI context write

**Errors:**

- `ProgramError::InvalidInstructionData` (error code: 3) - Failed to deserialize instruction data or invalid action configuration
- `ProgramError::InvalidAccountData` (error code: 4) - Account validation failures (wrong program ownership, invalid PDA derivation)
- `ProgramError::InvalidArgument` (error code: 1) - Invalid authority or action parameters
- `ErrorCode::MintActionProofMissing` (error code: 6070) - ZK proof required but not provided
- `ErrorCode::InvalidAuthorityMint` (error code: 6076) - Signer doesn't match mint authority
- `ErrorCode::MintActionAmountTooLarge` (error code: 6101) - Arithmetic overflow in mint amount calculations
- `ErrorCode::MintAccountMismatch` (error code: 6102) - SPL mint account doesn't match expected cmint
- `ErrorCode::InvalidAddressTree` (error code: 6069) - Wrong address merkle tree for mint creation
- `ErrorCode::MintActionMissingSplMintSigner` (error code: 6058) - Missing mint signer for SPL mint creation
- `ErrorCode::MintActionMissingMintAccount` (error code: 6061) - Missing SPL mint account when required
- `ErrorCode::MintActionMissingTokenPoolAccount` (error code: 6062) - Missing token pool PDA when required
- `ErrorCode::MintActionMissingTokenProgram` (error code: 6063) - Missing token program when required
- `ErrorCode::MintActionInvalidExtensionIndex` (error code: 6079) - Extension index out of bounds
- `ErrorCode::MintActionInvalidExtensionType` (error code: 6081) - Extension is not TokenMetadata type
- `ErrorCode::MintActionMetadataKeyNotFound` (error code: 6082) - Metadata key not found for removal
- `ErrorCode::MintActionMissingExecutingAccounts` (error code: 6083) - Missing required execution accounts
- `ErrorCode::CpiContextExpected` (error code: 6085) - CPI context required but not provided
- `AccountError::InvalidSigner` (error code: 12015) - Required signer account is not signing
- `AccountError::NotEnoughAccountKeys` (error code: 12020) - Missing required accounts
