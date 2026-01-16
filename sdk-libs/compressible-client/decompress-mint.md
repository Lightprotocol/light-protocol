# Decompress Mint SDK Design

## Overview

SDK-only functionality to decompress compressed CMint accounts (mints that were created via `#[compressible]` macro and have been auto-compressed by forester).

## Key Facts

### Can we build this purely in SDK without macro changes?

**YES**. This is fully supported by the existing `DecompressMint` instruction builder in `ctoken-sdk`.

**Why:**

1. **DecompressMint is permissionless** - the authority signer is required by the instruction format but NOT validated against `mint_authority`. Anyone can decompress any compressed mint.

2. **mint_seed does NOT need to sign** - uses `with_mint_signer_no_sign()` internally. The mint_seed is only used for PDA derivation.

3. `DecompressMint` struct already exists in `ctoken-sdk/src/ctoken/decompress_cmint.rs` with a complete `instruction()` method.

4. All data needed is queryable from the indexer via the compressed mint's address.

### Proof from on-chain code

From `programs/compressed-token/program/src/compressed_token/mint_action/actions/decompress_mint.rs`:

```rust
/// DecompressMint is **permissionless** - the caller pays initial rent, rent exemption is sponsored by the rent_sponsor.
/// The authority signer is still required for MintAction, but does not need to match mint_authority.
```

From `process_actions.rs` - DecompressMint does NOT call `check_authority()`:

```rust
ZAction::DecompressMint(decompress_action) => {
    // Note: No check_authority() call - permissionless!
    process_decompress_mint_action(
        decompress_action,
        compressed_mint,
        validated_accounts,
        mint_signer,
        fee_payer,
    )?;
}
```

### How program-created compressed mints work

When a mint is created via `#[compressible]` macro (like in `csdk-anchor-full-derived-test`):

1. **mint_seed_pubkey** = A program PDA (e.g., `LP_MINT_SIGNER_SEED + authority`)
2. **CMint PDA** = `find_mint_address(mint_seed_pubkey)` = PDA of `[COMPRESSED_MINT_SEED, mint_seed_pubkey]` under ctoken program
3. **Compressed address** = `derive_cmint_compressed_address(mint_seed_pubkey, address_tree)`

When querying the indexer:

- Query by `address = compressed_address` (derived from mint_seed_pubkey + address_tree)
- OR query by `cmint` pubkey if known

When decompressing:

- **Any signer can call** (permissionless)
- mint_seed_pubkey passed for CMint PDA derivation (does NOT sign)
- Fee payer pays for rent and top-up

## Architecture

### No CPI Required

Unlike PDAs which require program signing for decompression, CMint decompression is:

1. **Direct invoke** to ctoken program
2. **Permissionless** - any signer works as authority
3. **No custom seeds** needed from the caller program

### Existing Code Reuse

| Component                             | Location                                                                   | Reuse  |
| ------------------------------------- | -------------------------------------------------------------------------- | ------ |
| `DecompressMint` struct               | `ctoken-sdk/src/ctoken/decompress_cmint.rs`                                | Direct |
| `find_mint_address`                   | `ctoken-sdk/src/ctoken/create_cmint.rs`                                    | Direct |
| `derive_cmint_compressed_address`     | `ctoken-sdk/src/compressed_token/v2/create_compressed_mint/instruction.rs` | Direct |
| `CompressedMintWithContext`           | `ctoken-interface/src/instructions/mint_action/instruction_data.rs`        | Direct |
| `get_compressed_account`              | `light-client/src/indexer/indexer_trait.rs`                                | Direct |
| `get_validity_proof`                  | `light-client/src/indexer/indexer_trait.rs`                                | Direct |
| `config_pda()` / `rent_sponsor_pda()` | `ctoken-sdk/src/ctoken/mod.rs`                                             | Direct |

## API Design

### Input: `DecompressMintRequest`

```rust
#[derive(Debug, Clone)]
pub struct DecompressMintRequest {
    /// The seed pubkey used to derive the CMint PDA.
    /// This is the same value passed as `mint_signer` when the mint was created.
    pub mint_seed_pubkey: Pubkey,
    /// Address tree where the compressed mint was created.
    /// If None, uses the default cmint address tree.
    pub address_tree: Option<Pubkey>,
    /// Rent payment in epochs (must be 0 or >= 2). Default: 2
    pub rent_payment: Option<u8>,
    /// Lamports for future write operations. Default: 766
    pub write_top_up: Option<u32>,
}
```

### Primary Function Signature

```rust
/// Decompresses a compressed CMint to an on-chain CMint Solana account.
///
/// This is permissionless - any fee_payer can decompress any compressed mint.
/// The mint_seed_pubkey is used to derive the CMint PDA and compressed address.
///
/// # Arguments
/// * `request` - Decompress mint parameters
/// * `fee_payer` - Fee payer who pays rent and top-up
/// * `indexer` - Indexer for fetching compressed account and proof
///
/// # Returns
/// * Vec with decompress instruction if mint needs decompressing
/// * Empty vec if mint is already decompressed (idempotent)
/// * Error only for actual failures (not found, indexer errors)
pub async fn decompress_mint_idempotent<I: Indexer>(
    request: DecompressMintRequest,
    fee_payer: Pubkey,
    indexer: &I,
) -> Result<Vec<Instruction>, DecompressMintError>;
```

### Simplified API

For common case with defaults:

```rust
/// Decompress a compressed mint with default parameters.
///
/// Uses default address tree, rent_payment=2, write_top_up=766.
/// Returns empty vec if already decompressed (idempotent).
pub async fn decompress_mint<I: Indexer>(
    mint_seed_pubkey: Pubkey,
    fee_payer: Pubkey,
    indexer: &I,
) -> Result<Vec<Instruction>, DecompressMintError>;
```

### Error Type

```rust
#[derive(Debug, Error)]
pub enum DecompressMintError {
    #[error("Indexer error: {0}")]
    Indexer(#[from] IndexerError),

    #[error("Compressed mint not found for seed {mint_seed:?}")]
    MintNotFound { mint_seed: Pubkey },

    #[error("Missing compressed mint data in account")]
    MissingMintData,

    #[error("Program error: {0}")]
    ProgramError(#[from] ProgramError),
}
```

Note: `AlreadyDecompressed` is NOT an error - returns empty vec instead (idempotent behavior).

## Implementation

### File: `sdk-libs/compressible-client/src/decompress_mint.rs`

```rust
use light_client::indexer::{Indexer, IndexerError};
use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
use light_token_interface::{
    instructions::mint_action::{CompressedMintInstructionData, CompressedMintWithContext},
    state::CompressedMint,
    CMINT_ADDRESS_TREE,
};
use light_token_sdk::token::{
    derive_cmint_compressed_address, DecompressMint,
};
use solana_instruction::Instruction;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DecompressMintError {
    #[error("Indexer error: {0}")]
    Indexer(#[from] IndexerError),

    #[error("Compressed mint not found for seed {mint_seed:?}")]
    MintNotFound { mint_seed: Pubkey },

    #[error("Missing compressed mint data in account")]
    MissingMintData,

    #[error("Program error: {0}")]
    ProgramError(#[from] ProgramError),
}

#[derive(Debug, Clone)]
pub struct DecompressMintRequest {
    pub mint_seed_pubkey: Pubkey,
    pub address_tree: Option<Pubkey>,
    pub rent_payment: Option<u8>,
    pub write_top_up: Option<u32>,
}

impl DecompressMintRequest {
    pub fn new(mint_seed_pubkey: Pubkey) -> Self {
        Self {
            mint_seed_pubkey,
            address_tree: None,
            rent_payment: None,
            write_top_up: None,
        }
    }

    pub fn with_address_tree(mut self, address_tree: Pubkey) -> Self {
        self.address_tree = Some(address_tree);
        self
    }

    pub fn with_rent_payment(mut self, rent_payment: u8) -> Self {
        self.rent_payment = Some(rent_payment);
        self
    }

    pub fn with_write_top_up(mut self, write_top_up: u32) -> Self {
        self.write_top_up = Some(write_top_up);
        self
    }
}

/// Default rent payment in epochs (~24 hours per epoch)
pub const DEFAULT_RENT_PAYMENT: u8 = 2;
/// Default write top-up lamports (~3 hours rent per write)
pub const DEFAULT_WRITE_TOP_UP: u32 = 766;

/// Decompress a compressed mint with default parameters.
/// Returns empty vec if already decompressed (idempotent).
pub async fn decompress_mint<I: Indexer>(
    mint_seed_pubkey: Pubkey,
    fee_payer: Pubkey,
    indexer: &I,
) -> Result<Vec<Instruction>, DecompressMintError> {
    decompress_mint_idempotent(DecompressMintRequest::new(mint_seed_pubkey), fee_payer, indexer)
        .await
}

/// Decompresses a compressed CMint to an on-chain CMint Solana account.
///
/// This is permissionless - any fee_payer can decompress any compressed mint.
/// Returns empty vec if already decompressed (idempotent).
pub async fn decompress_mint_idempotent<I: Indexer>(
    request: DecompressMintRequest,
    fee_payer: Pubkey,
    indexer: &I,
) -> Result<Vec<Instruction>, DecompressMintError> {
    // 1. Derive addresses
    let address_tree = request
        .address_tree
        .unwrap_or(Pubkey::new_from_array(CMINT_ADDRESS_TREE));
    let compressed_address =
        derive_cmint_compressed_address(&request.mint_seed_pubkey, &address_tree);

    // 2. Fetch compressed mint account from indexer
    let compressed_account = indexer
        .get_compressed_account(compressed_address, None)
        .await?
        .value
        .ok_or(DecompressMintError::MintNotFound {
            mint_seed: request.mint_seed_pubkey,
        })?;

    // 3. Parse mint data from compressed account
    let mint_data = parse_compressed_mint_data(&compressed_account)?;

    // 4. Check if already decompressed - return empty vec (idempotent)
    if mint_data.metadata.cmint_decompressed {
        return Ok(vec![]);
    }

    // 5. Get validity proof
    let proof_result = indexer
        .get_validity_proof(vec![compressed_account.hash], vec![], None)
        .await?
        .value;

    // 6. Extract tree info from proof result
    let account_info = &proof_result.accounts[0];
    let state_tree = account_info.tree_info.tree;
    let input_queue = account_info.tree_info.queue;
    let output_queue = account_info
        .tree_info
        .next_tree_info
        .as_ref()
        .map(|next| next.queue)
        .unwrap_or(input_queue);

    // 7. Build CompressedMintWithContext
    let mint_instruction_data = CompressedMintInstructionData::try_from(mint_data)
        .map_err(|_| DecompressMintError::MissingMintData)?;

    let compressed_mint_with_context = CompressedMintWithContext {
        leaf_index: compressed_account.leaf_index,
        prove_by_index: compressed_account.prove_by_index,
        root_index: account_info
            .root_index
            .root_index()
            .unwrap_or_default(),
        address: compressed_address,
        mint: Some(mint_instruction_data),
    };

    // 8. Build DecompressMint instruction
    let decompress = DecompressMint {
        mint_seed_pubkey: request.mint_seed_pubkey,
        payer: fee_payer,
        authority: fee_payer, // Permissionless - any signer works
        state_tree,
        input_queue,
        output_queue,
        compressed_mint_with_context,
        proof: ValidityProof(proof_result.proof.into()),
        rent_payment: request.rent_payment.unwrap_or(DEFAULT_RENT_PAYMENT),
        write_top_up: request.write_top_up.unwrap_or(DEFAULT_WRITE_TOP_UP),
    };

    let ix = decompress.instruction().map_err(DecompressMintError::from)?;
    Ok(vec![ix])
}

/// Parse CompressedMint from compressed account data.
fn parse_compressed_mint_data(
    account: &light_client::indexer::CompressedAccount,
) -> Result<CompressedMint, DecompressMintError> {
    use borsh::BorshDeserialize;

    let data = account
        .data
        .as_ref()
        .ok_or(DecompressMintError::MissingMintData)?;

    CompressedMint::try_from_slice(&data.data)
        .map_err(|_| DecompressMintError::MissingMintData)
}
```

### Module Export

Update `sdk-libs/compressible-client/src/lib.rs`:

```rust
pub mod decompress_atas;
pub mod decompress_mint;
pub mod get_compressible_account;

pub use decompress_atas::*;
pub use decompress_mint::*;
```

## Flow Diagram

```
User calls decompress_mint(mint_seed_pubkey, fee_payer, indexer)
    |
    v
derive_cmint_compressed_address(mint_seed_pubkey, address_tree) -> compressed_address
    |
    v
indexer.get_compressed_account(compressed_address) -> CompressedAccount { data, hash, tree_info }
    |
    v
parse_compressed_mint_data() -> CompressedMint { metadata.cmint_decompressed? }
    |
    +--[if cmint_decompressed == true]--> Return empty vec (idempotent)
    |
    v  [if cmint_decompressed == false]
indexer.get_validity_proof([hash]) -> ValidityProofWithContext
    |
    v
DecompressMint {
    mint_seed_pubkey,
    payer: fee_payer,
    authority: fee_payer,  // Permissionless!
    ...
}.instruction() -> Instruction
    |
    v
Return vec![instruction] (caller signs and sends)
```

## Comparison with Other Decompress APIs

| Aspect          | Mint Decompress             | ATA Decompress     | PDA Decompress           |
| --------------- | --------------------------- | ------------------ | ------------------------ |
| Invoke type     | Direct invoke               | Direct invoke      | CPI from program         |
| Signing         | Permissionless (any signer) | Wallet owner signs | Program signs with seeds |
| Seed derivation | mint_seed -> CMint PDA      | Standard ATA       | Custom per-program       |
| Macro support   | No                          | No                 | Yes                      |
| Complexity      | Low                         | Low                | Higher                   |
| Authority check | None                        | ATA derivation     | PDA derivation           |

## Implementation Notes

### Why Permissionless?

Decompressing a mint doesn't change ownership or authority - it just brings the data back on-chain. The same mint_authority retains control. Anyone paying rent can "warm up" a cold mint.

### Idempotency

The function checks `cmint_decompressed` flag:

- If false: returns vec with decompress instruction
- If true: returns empty vec (nothing to do)

This follows the same pattern as `decompress_atas_idempotent` - callers can safely call multiple times without error handling.

### Transaction Execution

The returned instructions require **only the fee_payer to sign**:

```rust
let instructions = decompress_mint(mint_seed, fee_payer, &indexer).await?;
if !instructions.is_empty() {
    rpc.create_and_send_transaction(&instructions, &fee_payer, &[&fee_payer_keypair]).await?;
}
// If empty, mint was already decompressed - nothing to do
```

### Error Handling

- `MintNotFound`: Compressed mint doesn't exist (never created or wrong address_tree)
- `AlreadyDecompressed`: Mint is already on-chain (idempotent case)
- `MissingMintData`: Compressed account exists but has no mint data (shouldn't happen)

## Dependencies

```toml
# In sdk-libs/compressible-client/Cargo.toml
[dependencies]
light-client = { path = "../client" }
light-token-sdk = { path = "../ctoken-sdk" }
light-token-interface = { path = "../../program-libs/ctoken-interface" }
light-compressed-account = { path = "../../program-libs/compressed-account" }
solana-pubkey = "2"
solana-instruction = "2"
solana-program-error = "2"
thiserror = "1.0"
```

## Testing

### Test Cases

1. **Basic decompress**: Compressed mint -> on-chain CMint
2. **Already decompressed**: Returns `AlreadyDecompressed` error
3. **Not found**: Returns `MintNotFound` error
4. **Custom address tree**: Works with non-default address tree
5. **Custom rent/top-up**: Respects custom parameters

### Integration Test Pattern

```rust
#[tokio::test]
async fn test_decompress_mint() {
    // Setup: Create mint via program, warp to compress
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create mint via csdk-anchor-full-derived-test
    let (mint_signer_pda, _) = Pubkey::find_program_address(
        &[LP_MINT_SIGNER_SEED, authority.pubkey().as_ref()],
        &program_id,
    );
    let (cmint_pda, _) = find_mint_address(&mint_signer_pda);

    // ... execute create_pdas_and_mint_auto ...

    // Verify mint exists on-chain
    assert!(rpc.get_account(cmint_pda).await?.is_some());

    // Warp to auto-compress
    rpc.warp_slot_forward(SLOTS_PER_EPOCH * 30).await?;

    // Verify mint is compressed (closed on-chain)
    assert!(rpc.get_account(cmint_pda).await?.is_none());

    // DECOMPRESS using the new API
    let instructions = decompress_mint(mint_signer_pda, payer.pubkey(), &rpc).await?;
    assert_eq!(instructions.len(), 1); // Should have one decompress instruction

    // Execute (only fee_payer signs)
    rpc.create_and_send_transaction(&instructions, &payer.pubkey(), &[&payer]).await?;

    // Verify mint is back on-chain
    assert!(rpc.get_account(cmint_pda).await?.is_some());

    // Verify calling again returns empty vec (idempotent)
    let instructions = decompress_mint(mint_signer_pda, payer.pubkey(), &rpc).await?;
    assert!(instructions.is_empty()); // Already decompressed, nothing to do
}
```

## Files to Create/Modify

1. **Create**: `sdk-libs/compressible-client/src/decompress_mint.rs`
2. **Modify**: `sdk-libs/compressible-client/src/lib.rs` (add module export)
3. **Modify**: `sdk-libs/compressible-client/Cargo.toml` (add dependencies if missing)
4. **Test**: Add integration test to `sdk-tests/` directory
