## Compressed Token Freeze

**path:** `programs/compressed-token/anchor/src/freeze.rs`

**description:**
Freezes compressed token accounts. This instruction consumes input compressed token accounts (state: Initialized) and creates output compressed token accounts with state set to Frozen. The freeze authority (from mint) must sign the transaction, but the token account owner is NOT required to sign - the owner pubkey is provided in the instruction data and verified through proof verification.

Proof can be either a ZK proof or proof-by-index (when accounts are in an output queue of a batched merkle tree). When a default/zero `CompressedProof` is passed, the light system program is invoked with `None` as proof, enabling proof-by-index verification.

Frozen compressed token accounts cannot be transferred until thawed. The instruction preserves balances, delegates, and any TLV extensions from the input accounts.

Supports multiple hashing versions via an optional trailing version byte:
- V1 (1): Poseidon hash with little-endian amount bytes (default for backward compatibility)
- V2 (2): Poseidon hash with big-endian amount bytes
- ShaFlat (3): SHA256 hash (required for TLV extensions)

**Instruction data:**
`CompressedTokenInstructionDataFreeze` from `programs/compressed-token/anchor/src/freeze.rs:47`

| Field | Type | Description |
|-------|------|-------------|
| proof | CompressedProof | ZK proof for input account validity |
| owner | Pubkey | Owner of the token accounts (not required to sign) |
| input_token_data_with_context | Vec<InputTokenDataWithContext> | Input token data with merkle context |
| cpi_context | Option<CompressedCpiContext> | Optional CPI context for composability |
| outputs_merkle_tree_index | u8 | Index of output merkle tree in remaining accounts |
| (trailing byte) | Option<u8> | Optional version byte (1=V1, 2=V2, 3=ShaFlat) |

`InputTokenDataWithContext` fields:
- `amount: u64` - Token amount
- `delegate_index: Option<u8>` - Index of delegate in remaining accounts
- `merkle_context: PackedMerkleContext` - Merkle tree and queue indices
- `root_index: u16` - Root index for proof verification
- `lamports: Option<u64>` - Optional lamports attached to account
- `tlv: Option<Vec<ExtensionStruct>>` - TLV extensions (only with ShaFlat version)

**Accounts:**
`FreezeInstruction` from `programs/compressed-token/anchor/src/instructions/freeze.rs:12`

| # | Account | Type | Description |
|---|---------|------|-------------|
| 1 | fee_payer | Signer, Mutable | Pays transaction fees |
| 2 | authority | Signer | Must match mint's freeze_authority |
| 3 | cpi_authority_pda | PDA | Seeds: [CPI_AUTHORITY_PDA_SEED], program: self |
| 4 | light_system_program | Program | Light system program for compressed account operations |
| 5 | registered_program_pda | Account | Registered program PDA from light system program |
| 6 | noop_program | Account | Noop program for event emission |
| 7 | account_compression_authority | PDA | CPI authority for account compression program |
| 8 | account_compression_program | Program | Account compression program |
| 9 | self_program | Program | This program (cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m) |
| 10 | system_program | Program | System program |
| 11 | mint | Account | Token mint with freeze_authority set |
| + | remaining_accounts | Various | Merkle trees, queues, and delegate accounts |

**Instruction Logic and Checks:**

1. **Parse instruction data:**
   - Deserialize `CompressedTokenInstructionDataFreeze` from input bytes
   - Check for optional trailing version byte
   - Default to V1 if no version specified

2. **Validate inputs:**
   - Return `NoInputTokenAccountsProvided` if input_token_data_with_context is empty
   - Return `InvalidTokenDataVersion` if TLV is present without ShaFlat version

3. **Verify freeze authority:**
   - Anchor constraint verifies authority == mint.freeze_authority
   - Return `MintHasNoFreezeAuthority` if mint has no freeze authority
   - Return `InvalidFreezeAuthority` if authority doesn't match

4. **Build input compressed accounts:**
   - Call `get_input_compressed_accounts_with_merkle_context_and_check_signer::<false>` (FROZEN_INPUTS=false)
   - Reconstruct token data from inputs using owner from instruction data
   - Set input state to Initialized (expected input state)

5. **Build output compressed accounts:**
   - Call `create_token_output_accounts::<true>` (IS_FROZEN=true)
   - Create outputs with same amount, delegate, and TLV as inputs
   - Set output state to Frozen
   - Hash using specified version (V1, V2, or ShaFlat)
   - Set discriminator based on version

6. **Add data hash to inputs:**
   - Call `add_data_hash_to_input_compressed_accounts_with_version::<false>`
   - Hash inputs using specified version for proof verification

7. **Execute CPI:**
   - Call `cpi_execute_compressed_transaction_transfer` to light system program
   - Nullify input accounts and insert output accounts into merkle tree

**Errors:**

- `ErrorCode::NoInputTokenAccountsProvided` - No input token accounts provided
- `ErrorCode::InvalidTokenDataVersion` - TLV provided without ShaFlat version
- `ErrorCode::MintHasNoFreezeAuthority` - Mint's freeze_authority is None
- `ErrorCode::InvalidFreezeAuthority` - Authority doesn't match mint's freeze_authority
- Light system program errors from proof verification

**SDK:**
`freeze::sdk::create_freeze_instruction` from `programs/compressed-token/anchor/src/freeze.rs:349`

```rust
use light_compressed_token::freeze::sdk::{create_freeze_instruction, CreateInstructionInputs};

let instruction = create_freeze_instruction(CreateInstructionInputs {
    fee_payer,
    authority: freeze_authority,
    root_indices,
    proof,
    input_token_data,
    input_compressed_accounts,
    input_merkle_contexts,
    outputs_merkle_tree,
})?;
```
