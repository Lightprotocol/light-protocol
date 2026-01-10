## MintAction

**discriminator:** 103
**enum:** `InstructionType::MintAction`
**path:** programs/compressed-token/program/src/compressed_token/mint_action/

**description:**
Batch instruction for managing compressed mint accounts (cmints) and performing mint operations. A compressed mint account stores the mint's supply, decimals, authorities (mint/freeze), and optional TokenMetadata extension in compressed state. TokenMetadata is the only extension supported for compressed mints and provides fields for name, symbol, uri, update_authority, and additional key-value metadata.

This instruction supports 10 total actions - one creation action (controlled by `create_mint` flag) and 9 enum-based actions:

**Compressed mint creation (executed first when `create_mint` is Some):**

1. **Create Compressed Mint** - Create a new compressed mint account with initial authorities and optional TokenMetadata extension

**Core mint operations (Action enum variants):** 2. `MintToCompressed` - Mint new compressed tokens to one or more compressed token accounts 3. `MintToCToken` - Mint new tokens to decompressed ctoken accounts (not SPL tokens)

**Authority updates (Action enum variants):** 4. `UpdateMintAuthority` - Update or remove the mint authority 5. `UpdateFreezeAuthority` - Update or remove the freeze authority

**TokenMetadata extension operations (Action enum variants):** 6. `UpdateMetadataField` - Update name, symbol, uri, or additional_metadata fields in the TokenMetadata extension 7. `UpdateMetadataAuthority` - Update the metadata update authority in the TokenMetadata extension 8. `RemoveMetadataKey` - Remove a key-value pair from additional_metadata in the TokenMetadata extension

**Decompress/Compress operations (Action enum variants):** 9. `DecompressMint` - Decompress a compressed mint to a CMint Solana account. Creates a CMint PDA that becomes the source of truth. 10. `CompressAndCloseCMint` - Compress and close a CMint Solana account. Permissionless - anyone can call if is_compressible() returns true (rent expired).

Key concepts integrated:

- **Compressed mint (cmint)**: Mint state stored in compressed account with deterministic address derived from a mint signer PDA
- **Decompressed mint (CMint)**: When a compressed mint is decompressed, a CMint Solana account becomes the source of truth
- **Authority validation**: All actions require appropriate authority (mint/freeze/metadata) to be transaction signer
- **Batch processing**: Multiple actions execute sequentially with state updates persisted between actions

**Instruction data:**

1. instruction data is defined in path: program-libs/token-interface/src/instructions/mint_action/instruction_data.rs

   **Core fields:**
   - `leaf_index`: u32 - Merkle tree leaf index of existing compressed mint (only used if create_mint is None)
   - `prove_by_index`: bool - Use proof-by-index for existing mint validation (only used if create_mint is None)
   - `root_index`: u16 - Root index for address proof (create) or validity proof (update). Not used if proof by index.
   - `max_top_up`: u16 - Maximum lamports for rent and top-up combined. Transaction fails if exceeded. (0 = no limit)
   - `create_mint`: Option<CreateMint> - Configuration for creating new compressed mint (None for existing mint operations)
   - `actions`: Vec<Action> - Ordered list of actions to execute
   - `proof`: Option<CompressedProof> - ZK proof for compressed account validation (required unless prove_by_index=true)
   - `cpi_context`: Option<CpiContext> - For cross-program invocation support
   - `mint`: Option<MintInstructionData> - Full mint state including supply, decimals, metadata, authorities, and extensions (None when reading from decompressed CMint)

2. Action types (path: program-libs/token-interface/src/instructions/mint_action/):
   - `MintToCompressed(MintToCompressedAction)` - Mint tokens to compressed accounts (mint_to_compressed.rs)
   - `UpdateMintAuthority(UpdateAuthority)` - Update mint authority (update_mint.rs)
   - `UpdateFreezeAuthority(UpdateAuthority)` - Update freeze authority (update_mint.rs)
   - `MintToCToken(MintToCTokenAction)` - Mint to ctoken accounts (mint_to_ctoken.rs)
   - `UpdateMetadataField(UpdateMetadataFieldAction)` - Update metadata field (update_metadata.rs)
   - `UpdateMetadataAuthority(UpdateMetadataAuthorityAction)` - Update metadata authority (update_metadata.rs)
   - `RemoveMetadataKey(RemoveMetadataKeyAction)` - Remove metadata key (update_metadata.rs)
   - `DecompressMint(DecompressMintAction)` - Decompress compressed mint to CMint Solana account (decompress_mint.rs)
   - `CompressAndCloseCMint(CompressAndCloseCMintAction)` - Compress and close CMint Solana account (compress_and_close_cmint.rs)

**Accounts:**

The account ordering differs based on whether writing to CPI context or executing.

**Always present:**

1. light_system_program
   - non-mutable
   - Light Protocol system program for CPI to create or update the compressed mint account.

2. mint_signer (optional)
   - (signer if create_mint is Some)
   - Required only if create_mint is Some
   - PDA seed derivation from compressed mint randomness

3. authority
   - (signer)
   - Must match current mint/freeze/metadata authority for respective actions

**For execution (when not writing to CPI context):**

4. compressible_config (optional)
   - Required when DecompressMint or CompressAndCloseCMint action is present
   - CompressibleConfig account - parsed and validated for active state

5. cmint (optional)
   - (mutable) - CMint Solana account (decompressed compressed mint)
   - Required when cmint_decompressed=true OR DecompressMint OR CompressAndCloseCMint action present

6. rent_sponsor (optional)
   - (mutable) - Required when DecompressMint or CompressAndCloseCMint action is present
   - Rent sponsor PDA that pays for CMint account creation

7-12. Light system accounts (standard set):

- fee_payer (signer, mutable)
- cpi_authority_pda
- registered_program_pda
- account_compression_authority
- account_compression_program
- system_program
- sol_pool_pda (optional)
- sol_decompression_recipient (optional)
- cpi_context (optional)

13. out_output_queue

- (mutable)
- Output queue for compressed mint account updates

14. address_merkle_tree OR in_merkle_tree

- (mutable)
- If create_mint is Some: address_merkle_tree for new mint (must be CMINT_ADDRESS_TREE)
- If create_mint is None: in_merkle_tree for existing mint validation

15. in_output_queue

- (mutable) - optional, required if create_mint is None
- Input queue for existing compressed mint

16. tokens_out_queue

- (mutable) - optional, required for MintToCompressed actions
- Output queue for newly minted compressed token accounts

**For CPI context write (when write_to_cpi_context=true):**
4-6. CPI context accounts:

- fee_payer (signer, mutable)
- cpi_authority_pda
- cpi_context

**Packed accounts (remaining accounts):**

- Merkle tree and queue accounts for compressed storage
- Recipient ctoken accounts for MintToCToken action

**Instruction Logic and Checks:**

1. **Parse and validate instruction data:**
   - Deserialize `MintActionCompressedInstructionData` using zero-copy
   - Validate proof exists unless prove_by_index=true
   - Configure account requirements based on actions

2. **Validate and parse accounts:**
   - Check authority is signer
   - Validate CMint account matches expected mint pubkey (when cmint_pubkey provided)
   - For create_mint: validate address_merkle_tree is CMINT_ADDRESS_TREE
   - Parse compressible config when DecompressMint or CompressAndCloseCMint action present
   - Extract packed accounts for dynamic operations

3. **Process mint creation or input:**
   - If create_mint is Some:
     - Derive mint PDA from mint_signer key: `find_program_address([COMPRESSED_MINT_SEED, mint_signer], program_id)`
     - Validate mint.metadata.mint matches derived PDA
     - Validate compressed address derivation (especially with CPI context)
     - Set new address params in CPI instruction
   - If create_mint is None:
     - Hash existing compressed mint account
     - Set input with merkle context (tree, queue, leaf_index, proof)

4. **Process actions sequentially:**
   Each action validates authority and updates compressed mint state:

   **MintToCompressed:**
   - Validate: mint authority matches signer
   - Calculate: sum recipient amounts with overflow protection
   - Update: mint supply += sum_amounts
   - Create: compressed token accounts for each recipient

   **UpdateMintAuthority / UpdateFreezeAuthority:**
   - Validate: current authority matches signer
   - Update: set new authority (can be None to disable)

   **MintToCToken:**
   - Validate: mint authority matches signer
   - Calculate: sum recipient amount
   - Update: mint supply += amount
   - Update ctoken account balance via decompress operation

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

   **DecompressMint:**
   - Decompress compressed mint to a CMint Solana account
   - Create CMint PDA that becomes the source of truth
   - Update cmint_decompressed flag in compressed mint metadata

   **CompressAndCloseCMint:**
   - Compress and close a CMint Solana account
   - Permissionless - anyone can call if is_compressible() returns true (rent expired)
   - Compressed mint state is preserved

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
- `ProgramError::NotEnoughAccountKeys` - Missing required accounts
- `ErrorCode::MintActionProofMissing` (error code: 6055) - ZK proof required but not provided
- `ErrorCode::InvalidAuthorityMint` (error code: 6018) - Signer doesn't match mint authority
- `ErrorCode::MintActionAmountTooLarge` (error code: 6069) - Arithmetic overflow in mint amount calculations
- `ErrorCode::MintAccountMismatch` (error code: 6051) - CMint account doesn't match expected mint
- `ErrorCode::InvalidAddressTree` (error code: 6094) - Wrong address merkle tree for mint creation
- `ErrorCode::MintActionMissingMintSigner` (error code: 6108) - Missing mint signer account
- `ErrorCode::MintActionMissingCMintAccount` (error code: 6109) - Missing CMint account for decompress mint action
- `ErrorCode::MintActionInvalidExtensionIndex` (error code: 6059) - Extension index out of bounds
- `ErrorCode::MintActionInvalidExtensionType` (error code: 6062) - Extension is not TokenMetadata type
- `ErrorCode::MintActionMetadataKeyNotFound` (error code: 6063) - Metadata key not found for removal
- `ErrorCode::MintActionMissingExecutingAccounts` (error code: 6064) - Missing required execution accounts
- `ErrorCode::MintActionInvalidMintPda` (error code: 6066) - Invalid mint PDA derivation
- `ErrorCode::MintActionOutputSerializationFailed` (error code: 6068) - Account data serialization failed
- `ErrorCode::MintActionInvalidInitialSupply` (error code: 6070) - Initial supply must be 0 for new mint creation
- `ErrorCode::MintActionUnsupportedVersion` (error code: 6071) - Mint version not supported
- `ErrorCode::MintActionInvalidCompressionState` (error code: 6072) - New mint must start as compressed
- `ErrorCode::MintActionUnsupportedOperation` (error code: 6073) - Unsupported operation
- `ErrorCode::CpiContextExpected` (error code: 6085) - CPI context required but not provided
- `ErrorCode::TooManyCompressionTransfers` (error code: 6095) - Account index out of bounds for MintToCToken
- `ErrorCode::MintActionInvalidCpiContextForCreateMint` (error code: 6104) - Invalid CPI context for create mint operation
- `ErrorCode::MintActionInvalidCpiContextAddressTreePubkey` (error code: 6105) - Invalid address tree pubkey in CPI context
- `ErrorCode::MintActionInvalidCompressedMintAddress` (error code: 6103) - Invalid compressed mint address derivation
- `ErrorCode::MintDataRequired` (error code: 6125) - Mint data required in instruction when not decompressed
- `ErrorCode::CannotDecompressAndCloseInSameInstruction` (error code: 6123) - Cannot combine DecompressMint and CompressAndCloseCMint in same instruction
- `ErrorCode::CompressAndCloseCMintMustBeOnlyAction` (error code: 6169) - CompressAndCloseCMint must be the only action in the instruction
- `ErrorCode::CpiContextSetNotUsable` (error code: 6035) - Mint to ctokens or decompress mint not allowed when writing to CPI context
- `CTokenError::MaxTopUpExceeded` - Max top-up budget exceeded

### Spl mint migration

- cmint to spl mint migration is unimplemented and not planned.
- A way to support it in the future would require a new instruction that creates an spl mint in the mint pda solana account and mints the supply to the spl interface.
