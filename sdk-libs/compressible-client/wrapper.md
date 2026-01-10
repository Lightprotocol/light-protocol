# Unified Decompression Wrapper Specification

## Problem Statement

Clients need a single entry point to decompress mixed account types:

- **ATAs** (Associated Token Accounts) - compression_only tokens owned by ATA pubkey
- **Program-owned CTokens** - tokens owned by program PDAs
- **Program-owned PDAs** - compressed program state

Each type has different invocation patterns:
| Type | Invocation | Signer | Program Required |
|------|-----------|--------|------------------|
| ATA | Direct invoke to ctoken | wallet_owner | No |
| Program CToken | CPI from user program | program PDA | Yes |
| Program PDA | CPI from user program | program PDA | Yes |

## Decision Tree

```
                        ┌──────────────────────────────────────┐
                        │  What type of compressed account?     │
                        └───────────────────┬──────────────────┘
                                            │
            ┌───────────────────────────────┼───────────────────────────────┐
            │                               │                               │
            ▼                               ▼                               ▼
    ┌───────────────┐             ┌─────────────────┐            ┌──────────────────┐
    │     ATA       │             │  Program PDA    │            │ Program CToken   │
    │ (wallet owns  │             │  (program owns  │            │ (program PDA     │
    │  the tokens)  │             │   the state)    │            │  owns tokens)    │
    └───────┬───────┘             └────────┬────────┘            └────────┬─────────┘
            │                              │                              │
            ▼                              └──────────────┬───────────────┘
    ┌───────────────┐                                    │
    │  SDK-ONLY     │                                    ▼
    │               │                        ┌───────────────────────┐
    │ decompress_   │                        │   REQUIRES PROGRAM    │
    │ atas_         │                        │                       │
    │ idempotent()  │                        │ User must have on-    │
    │               │                        │ chain program with    │
    │ - No program  │                        │ decompress_accounts_  │
    │   deployment  │                        │ idempotent handler    │
    │ - Wallet      │                        │                       │
    │   signs       │                        │ - Program CPI         │
    │ - Direct      │                        │ - Program signs       │
    │   invoke      │                        │ - Needs T: Pack type  │
    └───────────────┘                        └───────────────────────┘
```

## System Architecture

```
                    ┌─────────────────────────────────┐
                    │    decompress_all()             │
                    │    Unified Entry Point          │
                    └───────────────┬─────────────────┘
                                    │
                    ┌───────────────┼───────────────┐
                    ▼               ▼               ▼
             ┌──────────┐   ┌──────────────┐   ┌──────────┐
             │   ATAs   │   │ Program PDAs │   │ Program  │
             │          │   │              │   │ CTokens  │
             └────┬─────┘   └──────┬───────┘   └────┬─────┘
                  │                │                │
                  ▼                └───────┬────────┘
        ┌─────────────────┐               │
        │ decompress_atas │               ▼
        │ _idempotent()   │    ┌─────────────────────────┐
        │                 │    │ decompress_accounts     │
        │ Direct invoke   │    │ _idempotent()           │
        │ to ctoken       │    │                         │
        └────────┬────────┘    │ CPI through user        │
                 │             │ program (requires       │
                 │             │ program_id + discrim)   │
                 ▼             └────────────┬────────────┘
        ┌─────────────────┐                │
        │ Transaction 1   │                ▼
        │ (SDK-only)      │     ┌─────────────────────────┐
        │                 │     │ Transaction 2           │
        │ create_ata...   │     │ (User Program CPI)      │
        │ decompress_ata  │     │                         │
        └─────────────────┘     │ decompress_pdas+tokens  │
                                └─────────────────────────┘
```

## Data Flow

```
┌──────────────────────────────────────────────────────────────────────────┐
│                           Client Input                                    │
│  DecompressRequest {                                                     │
│     kind: AccountKind,    // ATA | ProgramPda | ProgramCtoken            │
│     pubkey: Pubkey,       // The account identifier                      │
│     hash: Option<[u8;32]> // Optional: specific compressed hash          │
│  }                                                                       │
└─────────────────────────────────┬────────────────────────────────────────┘
                                  │
                                  ▼
┌──────────────────────────────────────────────────────────────────────────┐
│                    Phase 1: Classification                                │
│                                                                          │
│  for request in requests:                                                │
│    match request.kind {                                                  │
│      ATA { wallet_owner, mint } => ata_requests.push(...)                │
│      ProgramPda { program_id, seeds } => pda_requests.push(...)          │
│      ProgramCtoken { program_id, seeds } => ctoken_requests.push(...)    │
│    }                                                                     │
└─────────────────────────────────┬────────────────────────────────────────┘
                                  │
                                  ▼
┌──────────────────────────────────────────────────────────────────────────┐
│                    Phase 2: Query Indexer                                 │
│                                                                          │
│  ATAs:                                                                   │
│    indexer.get_compressed_token_accounts_by_owner(ata_pubkey, mint)      │
│                                                                          │
│  Program PDAs:                                                           │
│    indexer.get_compressed_account_by_address(derived_address)            │
│                                                                          │
│  Program CTokens:                                                        │
│    indexer.get_compressed_token_accounts_by_owner(pda_pubkey, mint)      │
└─────────────────────────────────┬────────────────────────────────────────┘
                                  │
                                  ▼
┌──────────────────────────────────────────────────────────────────────────┐
│                    Phase 3: Proof Generation                              │
│                                                                          │
│  ATA hashes -> get_validity_proof() -> ata_proof                         │
│  PDA + CToken hashes -> get_validity_proof() -> program_proof            │
│                                                                          │
│  Note: PDAs and CTokens share a proof because they're batched in         │
│  the same CPI call through the user program                              │
└─────────────────────────────────┬────────────────────────────────────────┘
                                  │
                                  ▼
┌──────────────────────────────────────────────────────────────────────────┐
│                    Phase 4: Instruction Building                          │
│                                                                          │
│  let mut instructions = Vec::new();                                      │
│                                                                          │
│  // ATAs: SDK-only, no program involvement                               │
│  if !ata_requests.is_empty() {                                           │
│    instructions.extend(decompress_atas_idempotent(...)?);                │
│  }                                                                       │
│                                                                          │
│  // Program accounts: requires CPI through user program                  │
│  if !pda_requests.is_empty() || !ctoken_requests.is_empty() {            │
│    let ix = decompress_accounts_idempotent(                              │
│      program_id,                                                         │
│      discriminator,  // User provides this                               │
│      ...                                                                 │
│    )?;                                                                   │
│    instructions.push(ix);                                                │
│  }                                                                       │
└─────────────────────────────────┬────────────────────────────────────────┘
                                  │
                                  ▼
┌──────────────────────────────────────────────────────────────────────────┐
│                         Output                                            │
│                                                                          │
│  DecompressResult {                                                      │
│    ata_instructions: Vec<Instruction>,      // Can be sent standalone    │
│    program_instructions: Vec<Instruction>,  // Requires user program     │
│  }                                                                       │
│                                                                          │
│  OR                                                                      │
│                                                                          │
│  Vec<TransactionBatch> where each batch can be a single tx               │
└──────────────────────────────────────────────────────────────────────────┘
```

## API Design

### Account Kind Enum

```rust
/// Identifies the type of compressed account for decompression
#[derive(Debug, Clone)]
pub enum AccountKind {
    /// ATA-owned compressed token (compression_only)
    /// Owner is the ATA pubkey derived from wallet_owner + mint
    /// Decompression is SDK-only (direct invoke to ctoken program)
    Ata {
        wallet_owner: Pubkey,
        mint: Pubkey,
    },

    /// Program-owned compressed PDA
    /// Requires CPI through user's program
    ProgramPda {
        /// The program that owns this account
        program_id: Pubkey,
        /// The PDA pubkey (destination for decompression)
        pda_pubkey: Pubkey,
    },

    /// Program-owned compressed token
    /// Owner is a PDA of user's program
    /// Requires CPI through user's program
    ProgramCtoken {
        /// The program that owns the PDA which owns the ctoken
        program_id: Pubkey,
        /// The PDA pubkey that owns the compressed token
        owner_pda: Pubkey,
        /// The token mint
        mint: Pubkey,
    },
}
```

### Request Structure

```rust
/// A request to decompress a specific compressed account
#[derive(Debug, Clone)]
pub struct DecompressRequest {
    /// The kind of account and its identifying information
    pub kind: AccountKind,

    /// Optional: specific compressed account hash(es) to decompress
    /// If None, decompresses ALL compressed accounts matching the kind
    pub hashes: Option<Vec<[u8; 32]>>,
}
```

### Program Config (for PDA/CToken operations)

```rust
/// Configuration for program-owned account decompression
/// Only needed if decompressing ProgramPda or ProgramCtoken accounts
#[derive(Debug, Clone)]
pub struct ProgramDecompressConfig {
    /// The program ID that owns the accounts
    pub program_id: Pubkey,

    /// The discriminator for decompress_accounts_idempotent instruction
    /// SHA256("global:decompress_accounts_idempotent")[..8]
    pub discriminator: [u8; 8],

    /// Account metas for the program's DecompressAccountsIdempotent accounts struct
    /// This is program-specific and must be provided by the client
    pub program_account_metas: Vec<AccountMeta>,

    /// Packed account data type deserializer
    /// Used to convert compressed account data to the program's variant type
    pub pack_fn: fn(&CompressedAccount) -> Result<PackedAccountData, Error>,
}
```

### Result Structure

```rust
/// Result of decompress_all operation
#[derive(Debug)]
pub struct DecompressResult {
    /// Instructions for ATA decompression (SDK-only, no program needed)
    /// These can be sent as a standalone transaction
    pub ata_instructions: Vec<Instruction>,

    /// Instructions for program-owned account decompression
    /// These require the user's program to be deployed
    /// Each inner Vec is a set of instructions that must go in the same tx
    pub program_instructions: Vec<Vec<Instruction>>,

    /// Accounts that were skipped (already decompressed or not found)
    pub skipped: Vec<SkippedAccount>,
}

#[derive(Debug)]
pub struct SkippedAccount {
    pub kind: AccountKind,
    pub reason: SkipReason,
}

#[derive(Debug)]
pub enum SkipReason {
    NotFound,
    AlreadyDecompressed,
    InvalidState,
}
```

### Main Entry Point

````rust
/// Unified decompression entry point
///
/// Given a list of accounts to decompress (with their kinds), generates
/// the appropriate instructions to decompress them.
///
/// # Arguments
/// * `requests` - List of decompression requests
/// * `fee_payer` - The fee payer for all transactions
/// * `program_config` - Required if any ProgramPda or ProgramCtoken requests exist
/// * `indexer` - Indexer for querying compressed state and proofs
///
/// # Returns
/// * `DecompressResult` containing categorized instructions
///
/// # Example
/// ```rust
/// let requests = vec![
///     DecompressRequest {
///         kind: AccountKind::Ata { wallet_owner, mint },
///         hashes: None, // decompress all
///     },
///     DecompressRequest {
///         kind: AccountKind::ProgramPda { program_id, pda_pubkey },
///         hashes: Some(vec![specific_hash]),
///     },
/// ];
///
/// let result = decompress_all(
///     &requests,
///     fee_payer,
///     Some(program_config), // needed for ProgramPda
///     &indexer,
/// ).await?;
///
/// // Send ATA instructions first (no dependencies)
/// rpc.send_transaction(result.ata_instructions);
///
/// // Send program instructions (requires user program)
/// for ix_batch in result.program_instructions {
///     rpc.send_transaction(ix_batch);
/// }
/// ```
pub async fn decompress_all<I: Indexer>(
    requests: &[DecompressRequest],
    fee_payer: Pubkey,
    program_config: Option<&ProgramDecompressConfig>,
    indexer: &I,
) -> Result<DecompressResult, DecompressError>
````

### Convenience Functions

```rust
/// Decompress only ATAs (simplified API for common case)
pub async fn decompress_only_atas<I: Indexer>(
    wallet_mints: &[(Pubkey, Pubkey)], // (wallet_owner, mint) pairs
    fee_payer: Pubkey,
    indexer: &I,
) -> Result<Vec<Instruction>, DecompressError> {
    let requests: Vec<_> = wallet_mints
        .iter()
        .map(|(wallet_owner, mint)| DecompressRequest {
            kind: AccountKind::Ata {
                wallet_owner: *wallet_owner,
                mint: *mint,
            },
            hashes: None,
        })
        .collect();

    let result = decompress_all(&requests, fee_payer, None, indexer).await?;
    Ok(result.ata_instructions)
}

/// Decompress program accounts with a pre-built config
pub async fn decompress_program_accounts<I: Indexer, T: Pack>(
    program_id: &Pubkey,
    discriminator: &[u8; 8],
    pda_pubkeys: &[Pubkey],
    program_account_metas: Vec<AccountMeta>,
    fee_payer: Pubkey,
    indexer: &I,
) -> Result<Instruction, DecompressError>
```

## Implementation Plan

### Phase 1: Core Types (wrapper_types.rs)

1. `AccountKind` enum
2. `DecompressRequest` struct
3. `ProgramDecompressConfig` struct
4. `DecompressResult` struct
5. `DecompressError` error enum

### Phase 2: Request Classification (wrapper.rs)

1. `classify_requests()` - separates ATAs from program accounts
2. `validate_program_config()` - ensures config exists when needed

### Phase 3: Indexer Queries

1. Batch ATA queries by wallet_owner
2. Batch PDA queries by program_id
3. Batch CToken queries by owner_pda

### Phase 4: Proof Generation

1. Collect all hashes per category
2. `get_validity_proof()` for ATA hashes
3. `get_validity_proof()` for program account hashes

### Phase 5: Instruction Building

1. Call existing `decompress_atas_idempotent()` for ATAs
2. Call existing `decompress_accounts_idempotent()` for program accounts

## Transaction Batching Considerations

```
┌────────────────────────────────────────────────────────────────────────┐
│                     Transaction Batching Rules                          │
│                                                                        │
│  1. ATAs: All in same tx (or split by compute limit)                   │
│     - create_ata_idempotent... (multiple)                              │
│     - decompress_batch (single ix, multiple inputs)                    │
│                                                                        │
│  2. Program PDAs + CTokens: All in same tx if same program             │
│     - decompress_accounts_idempotent(pda1, pda2, ctoken1, ctoken2)     │
│     - Order: PDAs first, then CTokens (CPI context handling)           │
│                                                                        │
│  3. Mixed programs: Separate transactions                              │
│     - Each program_id gets its own decompress_accounts_idempotent      │
│                                                                        │
│  4. Compute limits: May need to split large batches                    │
│     - ~200k CU per decompression                                       │
│     - ~1.4M CU limit per tx                                            │
│     - Max ~7 decompressions per tx                                     │
└────────────────────────────────────────────────────────────────────────┘
```

## Error Handling

```rust
#[derive(Debug, Error)]
pub enum DecompressError {
    #[error("Indexer error: {0}")]
    Indexer(#[from] IndexerError),

    #[error("CToken SDK error: {0}")]
    CTokenSdk(#[from] CTokenSdkError),

    #[error("Program config required for ProgramPda or ProgramCtoken accounts")]
    ProgramConfigRequired,

    #[error("Program ID mismatch: expected {expected}, got {got}")]
    ProgramIdMismatch { expected: Pubkey, got: Pubkey },

    #[error("No compressed accounts found for any request")]
    NoAccountsFound,

    #[error("Instruction building failed: {0}")]
    InstructionBuild(String),
}
```

## Usage Examples

### Example 1: Decompress User's ATAs Only

```rust
// User wants to decompress all their compressed USDC and SOL tokens
let result = decompress_all(
    &[
        DecompressRequest {
            kind: AccountKind::Ata {
                wallet_owner: user_wallet,
                mint: usdc_mint
            },
            hashes: None,
        },
        DecompressRequest {
            kind: AccountKind::Ata {
                wallet_owner: user_wallet,
                mint: wsol_mint
            },
            hashes: None,
        },
    ],
    user_wallet, // fee payer
    None, // no program config needed
    &indexer,
).await?;

// Send single transaction
send_transaction(result.ata_instructions).await?;
```

### Example 2: Decompress Game State (Mixed)

```rust
// Game has: user PDA (score), reward tokens (program-owned)
let program_config = ProgramDecompressConfig {
    program_id: game_program_id,
    discriminator: game::instruction::DecompressAccountsIdempotent::DISCRIMINATOR,
    program_account_metas: game::accounts::DecompressAccountsIdempotent {
        fee_payer: user_wallet,
        config: config_pda,
        rent_sponsor: rent_sponsor,
        // ... other accounts
    }.to_account_metas(None),
    pack_fn: |acc| GameAccountVariant::try_from(acc),
};

let result = decompress_all(
    &[
        // User's ATA (their own tokens)
        DecompressRequest {
            kind: AccountKind::Ata {
                wallet_owner: user_wallet,
                mint: reward_mint
            },
            hashes: None,
        },
        // Game PDA (score state)
        DecompressRequest {
            kind: AccountKind::ProgramPda {
                program_id: game_program_id,
                pda_pubkey: score_pda,
            },
            hashes: None,
        },
        // Program-owned reward tokens
        DecompressRequest {
            kind: AccountKind::ProgramCtoken {
                program_id: game_program_id,
                owner_pda: reward_vault_pda,
                mint: reward_mint,
            },
            hashes: None,
        },
    ],
    user_wallet,
    Some(&program_config),
    &indexer,
).await?;

// Transaction 1: ATAs (no program needed)
send_transaction(result.ata_instructions).await?;

// Transaction 2: Game state + program tokens (needs game program)
for ix_batch in result.program_instructions {
    send_transaction(ix_batch).await?;
}
```

## Key Constraints

1. **ATAs are SDK-only**: No program deployment needed, wallet signs directly
2. **Program accounts need CPI**: User must have deployed program with `decompress_accounts_idempotent`
3. **Proof batching**: Single proof for multiple accounts of same category
4. **CPI context ordering**: When mixing PDAs + CTokens, PDAs write first, CTokens consume last
5. **Program ID grouping**: Different programs = different transactions

## Files to Create/Modify

1. `sdk-libs/compressible-client/src/wrapper_types.rs` - Type definitions
2. `sdk-libs/compressible-client/src/wrapper.rs` - Main implementation
3. `sdk-libs/compressible-client/src/lib.rs` - Export new modules

## Dependencies

- Existing: `decompress_atas_idempotent` (just implemented)
- Existing: `compressible_instruction::decompress_accounts_idempotent`
- Existing: `light_client::indexer::Indexer` trait
- Existing: `light_sdk::compressible::Pack` trait

---

## Design Alternative: Simpler Approach

The main complexity in the unified wrapper comes from handling the generic `T: Pack` constraint for program-owned accounts. Each program defines its own `CompressedAccountVariant` enum.

### Alternative: Two-Tier API

Instead of one function that does everything, provide:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Two-Tier API                                        │
│                                                                             │
│  Tier 1: SDK-Only (no user program needed)                                  │
│  ─────────────────────────────────────────                                  │
│  decompress_atas_idempotent() - Already implemented                         │
│                                                                             │
│  Tier 2: Program-Aware (requires user program types)                        │
│  ───────────────────────────────────────────────────                        │
│  decompress_program_accounts<T: Pack>() - Generic over program variant      │
│                                                                             │
│  Combination Helper (convenience, not generic)                              │
│  ─────────────────────────────────────────────                              │
│  DecompressBuilder - Builder pattern for combining requests                 │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Builder Pattern API

```rust
/// Builder for constructing mixed decompression requests
pub struct DecompressBuilder<'a, I: Indexer> {
    indexer: &'a I,
    fee_payer: Pubkey,
    ata_requests: Vec<DecompressAtaRequest>,
    /// Program requests stored as pre-built instructions (caller handles T: Pack)
    program_instructions: Vec<Instruction>,
}

impl<'a, I: Indexer> DecompressBuilder<'a, I> {
    pub fn new(indexer: &'a I, fee_payer: Pubkey) -> Self {
        Self {
            indexer,
            fee_payer,
            ata_requests: Vec::new(),
            program_instructions: Vec::new(),
        }
    }

    /// Add an ATA to decompress
    pub fn add_ata(mut self, wallet_owner: Pubkey, mint: Pubkey) -> Self {
        self.ata_requests.push(DecompressAtaRequest {
            wallet_owner,
            mint,
            hashes: None,
        });
        self
    }

    /// Add multiple ATAs for same wallet
    pub fn add_atas(mut self, wallet_owner: Pubkey, mints: &[Pubkey]) -> Self {
        for mint in mints {
            self.ata_requests.push(DecompressAtaRequest {
                wallet_owner,
                mint: *mint,
                hashes: None,
            });
        }
        self
    }

    /// Add a pre-built program decompression instruction
    /// Caller is responsible for building this with correct T: Pack type
    pub fn add_program_instruction(mut self, instruction: Instruction) -> Self {
        self.program_instructions.push(instruction);
        self
    }

    /// Build all instructions
    pub async fn build(self) -> Result<DecompressResult, DecompressError> {
        let ata_instructions = if self.ata_requests.is_empty() {
            Vec::new()
        } else {
            decompress_atas_idempotent(&self.ata_requests, self.fee_payer, self.indexer).await?
        };

        Ok(DecompressResult {
            ata_instructions,
            program_instructions: self.program_instructions,
            skipped: Vec::new(),
        })
    }
}
```

### Usage with Builder

```rust
// Simple: ATAs only
let result = DecompressBuilder::new(&indexer, fee_payer)
    .add_ata(wallet, usdc_mint)
    .add_ata(wallet, wsol_mint)
    .build()
    .await?;

// Mixed: ATAs + program accounts
// Step 1: User builds their program instruction (they know the types)
let program_ix = compressible_instruction::decompress_accounts_idempotent::<GameVariant>(
    &game_program_id,
    &discriminator,
    &[pda1, pda2],
    &[(compressed_pda1, variant1), (compressed_pda2, variant2)],
    &program_account_metas,
    validity_proof,
)?;

// Step 2: Combine with builder
let result = DecompressBuilder::new(&indexer, fee_payer)
    .add_ata(wallet, reward_mint)
    .add_program_instruction(program_ix)
    .build()
    .await?;
```

### Why This is Better

1. **No phantom type complexity**: Caller handles `T: Pack` themselves
2. **No trait objects/dynamic dispatch**: Instructions are concrete
3. **Composable**: Easy to add more instruction types later
4. **Type safe**: Program-specific types stay in caller's code
5. **Simpler implementation**: Builder just aggregates, doesn't transform

---

## Recommended Implementation

Given the complexity of generic type handling across programs, the recommended implementation is:

### Core Functions (Already Exist / Just Built)

1. `decompress_atas_idempotent()` - SDK-only ATA decompression
2. `compressible_instruction::decompress_accounts_idempotent<T>()` - Program account decompression

### New Functions to Add

```rust
/// Convenience: Decompress all ATAs for a wallet across multiple mints
pub async fn decompress_wallet_atas<I: Indexer>(
    wallet_owner: Pubkey,
    mints: &[Pubkey],
    fee_payer: Pubkey,
    indexer: &I,
) -> Result<Vec<Instruction>, DecompressAtaError> {
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

/// Query helper: Find all compressed accounts for program-owned PDAs
/// Returns data needed to call decompress_accounts_idempotent
pub async fn query_program_compressed_accounts<I: Indexer>(
    program_id: &Pubkey,
    pda_pubkeys: &[Pubkey],
    indexer: &I,
) -> Result<QueryResult, DecompressError> {
    // Derives addresses, queries indexer, returns compressed accounts
    // Caller then deserializes data into their T type
}

/// Query helper: Find all compressed tokens owned by program PDAs
pub async fn query_program_compressed_tokens<I: Indexer>(
    owner_pdas: &[Pubkey],
    mint: Option<Pubkey>,
    indexer: &I,
) -> Result<Vec<CompressedTokenAccount>, DecompressError> {
    // Queries indexer for each owner PDA
}
```

### Final Recommended API

```
┌──────────────────────────────────────────────────────────────────────────┐
│                        Recommended API Surface                            │
│                                                                          │
│  HIGH-LEVEL (SDK-only)                                                   │
│  ─────────────────────                                                   │
│  decompress_atas_idempotent()      // Multiple ATAs, one proof           │
│  decompress_wallet_atas()          // All ATAs for wallet                │
│  decompress_all_for_ata()          // Single ATA convenience             │
│                                                                          │
│  QUERY HELPERS (for program accounts)                                    │
│  ─────────────────────────────────────                                   │
│  query_program_compressed_accounts()  // Find PDAs, return raw data      │
│  query_program_compressed_tokens()    // Find program-owned tokens       │
│                                                                          │
│  LOW-LEVEL (generic, caller provides types)                              │
│  ──────────────────────────────────────────                              │
│  decompress_accounts_idempotent<T>()  // Build instruction               │
│                                                                          │
│  BUILDER (composition)                                                   │
│  ─────────────────────                                                   │
│  DecompressBuilder                    // Combine ATAs + program ixs      │
└──────────────────────────────────────────────────────────────────────────┘
```

This keeps the API clean while acknowledging that program-specific type handling must stay with the caller.

---

## Complete End-to-End Flow Diagrams

### Scenario 1: User Decompresses Their Own ATAs (SDK-Only)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  User: "I want to decompress my USDC and SOL tokens"                        │
└──────────────────────────────────┬──────────────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Client Code                                                                │
│                                                                             │
│  let requests = vec![                                                       │
│      DecompressAtaRequest { wallet_owner, mint: usdc_mint, hashes: None },  │
│      DecompressAtaRequest { wallet_owner, mint: wsol_mint, hashes: None },  │
│  ];                                                                         │
│                                                                             │
│  let instructions = decompress_atas_idempotent(&requests, wallet, &ix)     │
│      .await?;                                                               │
└──────────────────────────────────┬──────────────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Internal: decompress_atas_idempotent                                       │
│                                                                             │
│  1. Derive ATA pubkeys: (wallet, usdc) -> ata1, (wallet, sol) -> ata2       │
│                                                                             │
│  2. Query indexer:                                                          │
│     indexer.get_compressed_token_accounts_by_owner(ata1, usdc)              │
│     indexer.get_compressed_token_accounts_by_owner(ata2, sol)               │
│                                                                             │
│  3. Get single proof for all hashes:                                        │
│     indexer.get_validity_proof([hash1, hash2, hash3...])                    │
│                                                                             │
│  4. Build instructions:                                                     │
│     - create_ata_idempotent(wallet, usdc)                                   │
│     - create_ata_idempotent(wallet, sol)                                    │
│     - transfer2(decompress all tokens to ATAs)                              │
└──────────────────────────────────┬──────────────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Single Transaction                                                         │
│                                                                             │
│  Instructions: [create_ata_usdc, create_ata_sol, decompress_batch]          │
│  Signers: [wallet]                                                          │
│                                                                             │
│  No on-chain program needed - direct invoke to ctoken program               │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Scenario 2: Game Decompresses Player State (Program Required)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  User: "I want to decompress my game score PDA and reward tokens"           │
└──────────────────────────────────┬──────────────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Client Code (Game-Specific)                                                │
│                                                                             │
│  // Step 1: Query compressed accounts                                       │
│  let score_address = derive_address(&score_pda, &tree, &game_id);           │
│  let score_account = indexer.get_compressed_account(score_address).await?;  │
│  let token_accounts = indexer                                               │
│      .get_compressed_token_accounts_by_owner(&reward_vault_pda, mint)       │
│      .await?;                                                               │
│                                                                             │
│  // Step 2: Deserialize into game's variant types                           │
│  let score_variant = GameVariant::Score(                                    │
│      ScoreData::deserialize(&score_account.data)?                           │
│  );                                                                         │
│  let token_variants: Vec<_> = token_accounts.iter()                         │
│      .map(|acc| GameVariant::Token(acc.token.clone()))                      │
│      .collect();                                                            │
│                                                                             │
│  // Step 3: Get proof for all accounts                                      │
│  let all_hashes = [score_account.hash].iter()                               │
│      .chain(token_accounts.iter().map(|a| a.account.hash))                  │
│      .collect();                                                            │
│  let proof = indexer.get_validity_proof(all_hashes, [], None).await?;       │
│                                                                             │
│  // Step 4: Build program instruction                                       │
│  let ix = decompress_accounts_idempotent::<GameVariant>(                    │
│      &game_program_id,                                                      │
│      &game::DECOMPRESS_DISCRIMINATOR,                                       │
│      &[score_pda, reward_vault_pda],                                        │
│      &[(score_account, score_variant), ...token_variants],                  │
│      &game_account_metas,                                                   │
│      proof,                                                                 │
│  )?;                                                                        │
└──────────────────────────────────┬──────────────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Single Transaction                                                         │
│                                                                             │
│  Instructions: [decompress_accounts_idempotent]                             │
│  Signers: [wallet]                                                          │
│                                                                             │
│  On-chain game program executes:                                            │
│  1. CPI to light-system-program (writes PDA to CPI context)                 │
│  2. CPI to ctoken-program (decompresses tokens, consumes CPI context)       │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Scenario 3: Mixed ATAs + Program Accounts

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  User: "Decompress my personal tokens AND my game state"                    │
└──────────────────────────────────┬──────────────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Client Code (Using Builder Pattern)                                        │
│                                                                             │
│  // Build ATAs                                                              │
│  let ata_ixs = decompress_atas_idempotent(&[                                │
│      DecompressAtaRequest { wallet_owner, mint: personal_token, ... },      │
│  ], wallet, &indexer).await?;                                               │
│                                                                             │
│  // Build program instruction (separate, with game types)                   │
│  let game_ix = build_game_decompress_ix(...)?;  // as shown in Scenario 2   │
│                                                                             │
│  // Combine using builder                                                   │
│  let result = DecompressBuilder::new(&indexer, wallet)                      │
│      .with_ata_instructions(ata_ixs)                                        │
│      .with_program_instruction(game_ix)                                     │
│      .build()?;                                                             │
└──────────────────────────────────┬──────────────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Two Transactions (or combined if compute allows)                           │
│                                                                             │
│  Transaction 1 (ATAs - SDK only):                                           │
│  - create_ata_idempotent                                                    │
│  - decompress_atas_batch                                                    │
│  - Signers: [wallet]                                                        │
│                                                                             │
│  Transaction 2 (Game - needs program):                                      │
│  - decompress_accounts_idempotent                                           │
│  - Signers: [wallet]                                                        │
│  - Program: game_program handles CPI                                        │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Implementation Checklist

### Already Implemented

- [x] `decompress_atas_idempotent` - SDK-only ATA decompression
- [x] `decompress_all_for_ata` - Single ATA convenience
- [x] `decompress_multiple_atas` - Simple multi-ATA wrapper
- [x] `compressible_instruction::decompress_accounts_idempotent<T>` - Program accounts

### To Implement (New Wrapper Layer)

1. **Query Helpers** (new file: `query_helpers.rs`)

```rust
// Find compressed PDAs by their derived addresses
pub async fn query_compressed_pdas<I: Indexer>(
    pda_pubkeys: &[Pubkey],
    address_tree: &Pubkey,
    program_id: &Pubkey,
    indexer: &I,
) -> Result<Vec<(Pubkey, CompressedAccount)>, DecompressError>;

// Find compressed tokens by owner PDAs
pub async fn query_compressed_tokens_by_owners<I: Indexer>(
    owner_pdas: &[Pubkey],
    mint: Option<Pubkey>,
    indexer: &I,
) -> Result<Vec<(Pubkey, Vec<CompressedTokenAccount>)>, DecompressError>;
```

2. **DecompressBuilder** (new file: `decompress_builder.rs`)

```rust
pub struct DecompressBuilder { ... }

impl DecompressBuilder {
    pub fn new(fee_payer: Pubkey) -> Self;
    pub fn with_ata_instructions(self, ixs: Vec<Instruction>) -> Self;
    pub fn with_program_instruction(self, ix: Instruction) -> Self;
    pub fn build(self) -> DecompressResult;
}
```

3. **Additional Convenience Functions** (add to `decompress_atas.rs`)

```rust
// Decompress all ATAs for a wallet
pub async fn decompress_wallet_atas<I: Indexer>(
    wallet: Pubkey,
    mints: &[Pubkey],
    fee_payer: Pubkey,
    indexer: &I,
) -> Result<Vec<Instruction>, DecompressAtaError>;
```

### Files to Create/Modify

| File                        | Action | Contents                            |
| --------------------------- | ------ | ----------------------------------- |
| `src/query_helpers.rs`      | Create | Query utilities for PDAs and tokens |
| `src/decompress_builder.rs` | Create | Builder for combining requests      |
| `src/decompress_atas.rs`    | Modify | Add `decompress_wallet_atas`        |
| `src/lib.rs`                | Modify | Export new modules                  |

---

## Summary

The recommended approach is a **two-tier API**:

1. **SDK-only tier** (for ATAs): Fully automatic, no program needed
2. **Program-aware tier**: Caller provides types, we provide query helpers

The builder pattern bridges both tiers for mixed use cases, keeping type safety while maintaining flexibility.

Key insight: We cannot fully abstract away the `T: Pack` generic without either:

- Runtime type erasure (losing type safety)
- Macro-generated code per program (complexity)

The builder pattern sidesteps this by letting callers handle their own types while we handle the orchestration.
