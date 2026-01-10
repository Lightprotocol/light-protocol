# Decompress ATAs Idempotent Design

## Overview

This document describes the SDK-only functionality to decompress multiple ATA-owned compressed token accounts in a single instruction with one proof.

## Key Facts

### Can we decompress multiple ATAs in one instruction with one proof?

**YES**. This is fully supported by the existing `transfer2` instruction.

**Why:**

1. `get_validity_proof(hashes: Vec<Hash>, ...)` accepts multiple account hashes and returns a single ZK proof covering all
2. `decompress_full_ctoken_accounts_with_indices` in `ctoken-sdk` already accepts `&[DecompressFullIndices]` for batched decompress
3. The `is_ata: bool` flag in `DecompressFullIndices` handles the ATA case correctly (owner is not marked as signer)

### How ATA-owned compressed tokens work

When a CToken ATA is auto-compressed:

- The compressed token's `owner` = ATA pubkey (not wallet owner)
- `CompressedOnlyExtension.is_ata = 1` marks it as ATA-owned
- Stored in TLV: `ExtensionStruct::CompressedOnly(CompressedOnlyExtension { is_ata: 1, ... })`

When querying the indexer:

- Query by `owner = ATA_pubkey` (not wallet owner)
- ATA pubkey = `derive_ctoken_ata(wallet_owner, mint)` = PDA of `[wallet_owner, CTOKEN_PROGRAM_ID, mint]`

When decompressing:

- Wallet owner signs the transaction (not the ATA, which is a PDA)
- `is_ata: true` in `DecompressFullIndices` ensures owner index is NOT marked as signer
- Program verifies ATA derivation: `derive_ctoken_ata(signer, mint) == compressed_owner`

## Architecture

### No Macro Support Required

This is purely SDK/client-side functionality because:

1. Direct invoke to ctoken program (no CPI from custom program)
2. Wallet owner signs (no program signing/seeds needed)
3. Standard ATA derivation (no custom seeds)
4. Existing `transfer2` instruction handles everything

### Existing Code Reuse

| Component               | Location                                                 | Reuse  |
| ----------------------- | -------------------------------------------------------- | ------ |
| ATA derivation          | `ctoken-sdk/src/ctoken/create_ata.rs::derive_ctoken_ata` | Direct |
| Decompress full indices | `ctoken-sdk/src/compressed_token/v2/decompress_full.rs`  | Direct |
| ATA packing             | `pack_for_decompress_full_with_ata`                      | Direct |
| Transfer2 instruction   | `create_transfer2_instruction`                           | Direct |
| Create ATA idempotent   | `CreateAssociatedCTokenAccount::idempotent()`            | Direct |
| Validity proof          | `light-client::Indexer::get_validity_proof`              | Direct |
| Token account query     | `get_compressed_token_accounts_by_owner`                 | Direct |

## API Design

### Input: `DecompressAtaRequest`

```rust
pub struct DecompressAtaRequest {
    /// Wallet owner (signer of the transaction)
    pub wallet_owner: Pubkey,
    /// Token mint
    pub mint: Pubkey,
    /// Optional: specific compressed token account hashes to decompress
    /// If None, decompress all compressed tokens for this ATA
    pub hashes: Option<Vec<[u8; 32]>>,
}
```

### Function Signature

```rust
/// Decompresses multiple ATA-owned compressed tokens in one instruction.
///
/// For each (wallet_owner, mint) pair:
/// 1. Derives the ATA address
/// 2. Fetches compressed token accounts owned by that ATA
/// 3. Gets a single validity proof for all accounts
/// 4. Creates destination ATAs if needed (idempotent)
/// 5. Builds single decompress instruction
///
/// # Arguments
/// * `requests` - List of (wallet_owner, mint) pairs to decompress
/// * `fee_payer` - Fee payer pubkey
/// * `indexer` - Indexer for fetching accounts and proofs
///
/// # Returns
/// * Vec of instructions: [create_ata_idempotent..., decompress_all]
/// * Returns empty vec if no compressed tokens found
pub async fn decompress_atas_idempotent<I: Indexer>(
    requests: &[DecompressAtaRequest],
    fee_payer: Pubkey,
    indexer: &I,
) -> Result<Vec<Instruction>, CompressibleClientError>;
```

### Batching Rules

1. **Single wallet, multiple mints**: Each mint requires separate ATA, but can share proof
2. **Multiple wallets**: Each wallet must sign, so typically separate transactions
3. **Same ATA, multiple compressed accounts**: Batched into single instruction (common case)

The common use case is: user has one wallet, multiple compressed token accounts under same ATA, wants to decompress all.

## Implementation Plan

### Step 1: Add to `light-compressible-client`

```rust
// In sdk-libs/compressible-client/src/lib.rs

pub mod decompress_atas;
pub use decompress_atas::*;
```

### Step 2: Core Implementation

The implementation follows the same pattern as `DecompressToCtoken::instruction()` in `ctoken-sdk/src/ctoken/decompress.rs`:

```rust
// sdk-libs/compressible-client/src/decompress_atas.rs

use light_client::indexer::{CompressedTokenAccount, Indexer, ValidityProofWithContext};
use light_compressed_account::compressed_account::PackedMerkleContext;
use light_ctoken_interface::{
    instructions::{
        extensions::{CompressedOnlyExtensionInstructionData, ExtensionInstructionData},
        transfer2::MultiInputTokenDataWithContext,
    },
    state::{ExtensionStruct, TokenDataVersion},
};
use light_ctoken_sdk::{
    compressed_token::{
        v2::transfer2::{
            create_transfer2_instruction, Transfer2AccountsMetaConfig, Transfer2Config,
            Transfer2Inputs,
        },
        CTokenAccount2,
    },
    compat::AccountState,
    ctoken::{derive_ctoken_ata, CreateAssociatedCTokenAccount},
};
use light_sdk::instruction::{PackedAccounts, PackedStateTreeInfo, ValidityProof};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

#[derive(Debug, Clone)]
pub struct DecompressAtaRequest {
    pub wallet_owner: Pubkey,
    pub mint: Pubkey,
    /// Optional: specific hashes to decompress. If None, decompress all.
    pub hashes: Option<Vec<[u8; 32]>>,
}

/// Decompresses multiple ATA-owned compressed tokens in one instruction.
///
/// Returns (create_ata_instructions, decompress_instruction).
/// The decompress instruction is None if no compressed tokens found.
pub async fn decompress_atas_idempotent<I: Indexer>(
    requests: &[DecompressAtaRequest],
    fee_payer: Pubkey,
    indexer: &I,
) -> Result<Vec<Instruction>, CompressibleClientError> {
    let mut create_ata_instructions = Vec::new();
    let mut all_accounts: Vec<AtaDecompressContext> = Vec::new();

    // Phase 1: Gather compressed token accounts and prepare ATA creation
    for request in requests {
        let (ata_pubkey, ata_bump) = derive_ctoken_ata(&request.wallet_owner, &request.mint);

        // Query compressed tokens owned by this ATA
        let result = indexer
            .get_compressed_token_accounts_by_owner(&ata_pubkey, None, None)
            .await?;

        let mut accounts = result.value.items;
        if accounts.is_empty() {
            continue;
        }

        // Filter by hashes if specified
        if let Some(ref hashes) = request.hashes {
            accounts.retain(|acc| hashes.contains(&acc.account.hash));
        }

        if accounts.is_empty() {
            continue;
        }

        // Create ATA idempotently
        let create_ata = CreateAssociatedCTokenAccount::new(
            fee_payer,
            request.wallet_owner,
            request.mint,
        ).idempotent().instruction()?;
        create_ata_instructions.push(create_ata);

        // Collect context for each account
        for acc in accounts {
            all_accounts.push(AtaDecompressContext {
                token_account: acc,
                ata_pubkey,
                wallet_owner: request.wallet_owner,
                ata_bump,
            });
        }
    }

    if all_accounts.is_empty() {
        return Ok(create_ata_instructions);
    }

    // Phase 2: Get validity proof for all accounts
    let hashes: Vec<[u8; 32]> = all_accounts
        .iter()
        .map(|ctx| ctx.token_account.account.hash)
        .collect();

    let proof_result = indexer
        .get_validity_proof(hashes, vec![], None)
        .await?
        .value;

    // Phase 3: Build decompress instruction
    let decompress_ix = build_batch_decompress_instruction(
        fee_payer,
        &all_accounts,
        proof_result,
    )?;

    let mut instructions = create_ata_instructions;
    instructions.push(decompress_ix);
    Ok(instructions)
}

struct AtaDecompressContext {
    token_account: CompressedTokenAccount,
    ata_pubkey: Pubkey,
    wallet_owner: Pubkey,
    ata_bump: u8,
}

fn build_batch_decompress_instruction(
    fee_payer: Pubkey,
    accounts: &[AtaDecompressContext],
    proof: ValidityProofWithContext,
) -> Result<Instruction, CompressibleClientError> {
    let mut packed_accounts = PackedAccounts::default();

    // Pack tree infos first (inserts trees and queues)
    let packed_tree_infos = proof.pack_tree_infos(&mut packed_accounts);
    let tree_infos = packed_tree_infos.state_trees.as_ref()
        .ok_or(CompressibleClientError::NoStateTreesInProof)?;

    let mut token_accounts_vec = Vec::with_capacity(accounts.len());
    let mut in_tlv_data: Vec<Vec<ExtensionInstructionData>> = Vec::with_capacity(accounts.len());
    let mut has_any_tlv = false;

    for (i, ctx) in accounts.iter().enumerate() {
        let token = &ctx.token_account.token;
        let account = &ctx.token_account.account;
        let tree_info = &tree_infos.packed_tree_infos[i];

        // Insert wallet_owner as signer (for ATA, wallet signs, not ATA pubkey)
        let owner_index = packed_accounts.insert_or_get_config(ctx.wallet_owner, true, false);

        // Insert ATA pubkey (as the token owner in TokenData - not a signer!)
        let ata_index = packed_accounts.insert_or_get(ctx.ata_pubkey);

        // Insert mint
        let mint_index = packed_accounts.insert_or_get(token.mint);

        // Insert delegate if present
        let delegate_index = token.delegate
            .map(|d| packed_accounts.insert_or_get(d))
            .unwrap_or(0);

        // Insert destination ATA
        let destination_index = packed_accounts.insert_or_get(ctx.ata_pubkey);

        // Build MultiInputTokenDataWithContext
        let source = MultiInputTokenDataWithContext {
            owner: ata_index,  // Token owner is ATA pubkey (not wallet!)
            amount: token.amount,
            has_delegate: token.delegate.is_some(),
            delegate: delegate_index,
            mint: mint_index,
            version: TokenDataVersion::ShaFlat as u8,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
                queue_pubkey_index: tree_info.queue_pubkey_index,
                prove_by_index: account.prove_by_index,
                leaf_index: account.leaf_index,
            },
            root_index: tree_info.root_index,
        };

        // Build CTokenAccount2 for decompress
        let mut ctoken_account = CTokenAccount2::new(vec![source.clone()])?;
        ctoken_account.decompress_ctoken(token.amount, destination_index)?;
        token_accounts_vec.push(ctoken_account);

        // Build TLV for this input (CompressedOnly extension for ATAs)
        let is_frozen = token.state == AccountState::Frozen;
        let tlv_vec: Vec<ExtensionInstructionData> = token.tlv.as_ref()
            .map(|exts| {
                exts.iter().filter_map(|ext| match ext {
                    ExtensionStruct::CompressedOnly(co) => {
                        Some(ExtensionInstructionData::CompressedOnly(
                            CompressedOnlyExtensionInstructionData {
                                delegated_amount: co.delegated_amount,
                                withheld_transfer_fee: co.withheld_transfer_fee,
                                is_frozen,
                                compression_index: 0,
                                is_ata: true,
                                bump: ctx.ata_bump,
                                owner_index,  // Wallet owner who signs
                            }
                        ))
                    }
                    _ => None,
                }).collect()
            })
            .unwrap_or_default();

        if !tlv_vec.is_empty() {
            has_any_tlv = true;
        }
        in_tlv_data.push(tlv_vec);
    }

    // Convert packed_accounts to AccountMetas
    let (packed_account_metas, _, _) = packed_accounts.to_account_metas();

    // Build Transfer2 instruction
    let meta_config = Transfer2AccountsMetaConfig::new(fee_payer, packed_account_metas);
    let transfer_config = Transfer2Config::default().filter_zero_amount_outputs();

    let inputs = Transfer2Inputs {
        meta_config,
        token_accounts: token_accounts_vec,
        transfer_config,
        validity_proof: proof.proof,
        in_tlv: if has_any_tlv { Some(in_tlv_data) } else { None },
        ..Default::default()
    };

    create_transfer2_instruction(inputs).map_err(CompressibleClientError::from)
}

#[derive(Debug)]
pub enum CompressibleClientError {
    Indexer(light_client::indexer::IndexerError),
    CTokenSdk(light_ctoken_sdk::error::CTokenSdkError),
    NoStateTreesInProof,
    ProgramError(solana_program_error::ProgramError),
}

impl From<light_client::indexer::IndexerError> for CompressibleClientError {
    fn from(e: light_client::indexer::IndexerError) -> Self {
        Self::Indexer(e)
    }
}

impl From<light_ctoken_sdk::error::CTokenSdkError> for CompressibleClientError {
    fn from(e: light_ctoken_sdk::error::CTokenSdkError) -> Self {
        Self::CTokenSdk(e)
    }
}

impl From<solana_program_error::ProgramError> for CompressibleClientError {
    fn from(e: solana_program_error::ProgramError) -> Self {
        Self::ProgramError(e)
    }
}

impl std::fmt::Display for CompressibleClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Indexer(e) => write!(f, "Indexer error: {:?}", e),
            Self::CTokenSdk(e) => write!(f, "CToken SDK error: {:?}", e),
            Self::NoStateTreesInProof => write!(f, "No state trees in proof"),
            Self::ProgramError(e) => write!(f, "Program error: {:?}", e),
        }
    }
}

impl std::error::Error for CompressibleClientError {}
```

### Step 3: Simplified Client API

For the common case (single wallet, all compressed tokens for an ATA):

```rust
/// Decompress all compressed tokens for a wallet's ATA
pub async fn decompress_all_for_ata<I: Indexer>(
    wallet_owner: Pubkey,
    mint: Pubkey,
    fee_payer: Pubkey,
    indexer: &I,
) -> Result<Vec<Instruction>, CompressibleClientError> {
    decompress_atas_idempotent(
        &[DecompressAtaRequest {
            wallet_owner,
            mint,
            hashes: None,
        }],
        fee_payer,
        indexer,
    ).await
}

/// Decompress multiple ATAs for multiple mints in one transaction
pub async fn decompress_multiple_atas<I: Indexer>(
    wallet_owner: Pubkey,
    mints: &[Pubkey],
    fee_payer: Pubkey,
    indexer: &I,
) -> Result<Vec<Instruction>, CompressibleClientError> {
    let requests: Vec<_> = mints
        .iter()
        .map(|mint| DecompressAtaRequest {
            wallet_owner,
            mint: *mint,
            hashes: None,
        })
        .collect();

    decompress_atas_idempotent(&requests, fee_payer, indexer).await
}
```

## Flow Diagram

```
User calls decompress_atas_idempotent([{wallet_owner, mint}])
    |
    v
derive_ctoken_ata(wallet_owner, mint) -> ata_pubkey
    |
    v
indexer.get_compressed_token_accounts_by_owner(ata_pubkey)
    |
    v
[CompressedTokenAccount { owner: ata_pubkey, is_ata: true, ... }]
    |
    v
indexer.get_validity_proof([hash1, hash2, ...]) -> single proof
    |
    v
CreateAssociatedCTokenAccount::idempotent() -> create_ata_ix
    |
    v
decompress_full_ctoken_accounts_with_indices(proof, indices) -> decompress_ix
    |
    v
Return [create_ata_ix, decompress_ix]
```

## Implementation Notes

### Key Implementation Insight

For ATA decompress, the compressed token's `owner` field contains the ATA pubkey (not the wallet owner). However:

1. **The wallet owner signs** the transaction (ATAs are PDAs that cannot sign)
2. **The ATA pubkey goes into TokenData.owner** (for merkle proof verification)
3. **The wallet_owner goes into CompressedOnlyExtension.owner_index** (for ATA derivation verification)

The ctoken program verifies: `derive_ctoken_ata(owner_from_owner_index, mint) == token_data.owner`

### Client vs On-chain Distinction

The implementation uses `create_transfer2_instruction` directly with `Transfer2Inputs` and `CTokenAccount2`, following the same pattern as `DecompressToCtoken::instruction()` in `ctoken-sdk/src/ctoken/decompress.rs`.

Key differences from on-chain `decompress_full_ctoken_accounts_with_indices`:

- Uses pubkeys instead of AccountInfo
- Builds AccountMetas via `PackedAccounts::to_account_metas()`
- No CPI needed (direct invoke)

### Error Handling

- Return empty vec if no compressed tokens found (idempotent)
- Fail if proof generation fails
- Fail if any individual decompress fails validation

### Transaction Size Limits

- Each compressed account adds ~100-150 bytes to instruction data
- Practical limit: ~15-20 accounts per instruction
- For more accounts: split into multiple instructions (still fewer transactions than individual decompress)

## Testing

### Test Cases

1. Single ATA with single compressed token
2. Single ATA with multiple compressed tokens (merge-like)
3. Multiple ATAs for same wallet, different mints
4. ATA already exists (idempotent create)
5. No compressed tokens (returns empty)
6. Mixed: some ATAs have tokens, some don't

### Example Test

```rust
#[tokio::test]
async fn test_decompress_ata_idempotent() {
    // Setup: Create ATA, mint tokens, warp to compress
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let mint = /* create mint */;

    // Create ATA and mint some tokens
    let (ata_pubkey, _) = derive_ctoken_ata(&payer.pubkey(), &mint);
    CreateAssociatedCTokenAccount::new(payer.pubkey(), payer.pubkey(), mint)
        .instruction()?.execute(&mut rpc).await?;

    // Mint tokens to ATA
    CTokenMintTo { ... }.invoke()?;

    // Warp to auto-compress
    rpc.warp_slot_forward(SLOTS_PER_EPOCH * 30).await?;

    // Verify ATA is closed (compressed)
    assert!(rpc.get_account(ata_pubkey).await?.is_none());

    // Verify compressed token exists owned by ATA pubkey
    let compressed = rpc.get_compressed_token_accounts_by_owner(&ata_pubkey, None, None).await?;
    assert_eq!(compressed.value.items.len(), 1);
    assert_eq!(compressed.value.items[0].token.owner, ata_pubkey);

    // DECOMPRESS using the new API
    let instructions = decompress_atas_idempotent(
        &[DecompressAtaRequest {
            wallet_owner: payer.pubkey(),
            mint,
            hashes: None,
        }],
        payer.pubkey(),
        &rpc,
    ).await?;

    // Execute
    rpc.create_and_send_transaction(&instructions, &payer.pubkey(), &[&payer]).await?;

    // Verify ATA is back on-chain with balance
    let ata_account = rpc.get_account(ata_pubkey).await?.unwrap();
    let ctoken = CToken::deserialize(&mut &ata_account.data[..])?;
    assert_eq!(ctoken.amount, expected_amount);

    // Verify no more compressed tokens
    let remaining = rpc.get_compressed_token_accounts_by_owner(&ata_pubkey, None, None).await?;
    assert!(remaining.value.items.is_empty());
}
```

## Comparison with PDA Decompress

| Aspect               | ATA Decompress     | PDA Decompress           |
| -------------------- | ------------------ | ------------------------ |
| Invoke type          | Direct invoke      | CPI from program         |
| Signing              | Wallet owner signs | Program signs with seeds |
| Seed derivation      | Standard ATA       | Custom per-program       |
| Macro support needed | No                 | Yes                      |
| Complexity           | Lower              | Higher                   |

## Files to Create/Modify

1. **Create**: `sdk-libs/compressible-client/src/decompress_atas.rs`
2. **Modify**: `sdk-libs/compressible-client/src/lib.rs` (add module export)
3. **Test**: Add test in `sdk-tests/` directory

## Dependencies

```toml
# In sdk-libs/compressible-client/Cargo.toml
[dependencies]
light-client = { path = "../client" }
light-ctoken-sdk = { path = "../ctoken-sdk" }
light-ctoken-interface = { path = "../../program-libs/ctoken-interface" }
light-compressed-account = { path = "../../program-libs/compressed-account" }
light-sdk = { path = "../sdk" }
solana-pubkey = "2"
solana-instruction = "2"
solana-program-error = "2"
```

## What Already Exists vs What to Create

### Already Exists (Reuse)

| Function                                                            | Location                                                      | Purpose                |
| ------------------------------------------------------------------- | ------------------------------------------------------------- | ---------------------- |
| `derive_ctoken_ata`                                                 | `ctoken-sdk/src/ctoken/create_ata.rs`                         | Derive ATA address     |
| `CreateAssociatedCTokenAccount::idempotent()`                       | `ctoken-sdk/src/ctoken/create_ata.rs`                         | Create ATA instruction |
| `create_transfer2_instruction`                                      | `ctoken-sdk/src/compressed_token/v2/transfer2/instruction.rs` | Build transfer2 ix     |
| `CTokenAccount2::decompress_ctoken`                                 | `ctoken-sdk/src/compressed_token/v2/account2.rs`              | Set decompress mode    |
| `ValidityProofWithContext::pack_tree_infos`                         | `light-client/src/indexer/types.rs`                           | Pack tree info         |
| `PackedAccounts`                                                    | `light-sdk/src/instruction/packed_accounts.rs`                | Account packing        |
| `Transfer2Inputs`, `Transfer2Config`, `Transfer2AccountsMetaConfig` | `ctoken-sdk/src/compressed_token/v2/transfer2/`               | Transfer2 config       |

### To Create

| Function                     | Location                                     | Purpose         |
| ---------------------------- | -------------------------------------------- | --------------- |
| `decompress_atas_idempotent` | `compressible-client/src/decompress_atas.rs` | Main API        |
| `decompress_all_for_ata`     | `compressible-client/src/decompress_atas.rs` | Convenience API |
| `decompress_multiple_atas`   | `compressible-client/src/decompress_atas.rs` | Multi-mint API  |
| `CompressibleClientError`    | `compressible-client/src/decompress_atas.rs` | Error type      |
