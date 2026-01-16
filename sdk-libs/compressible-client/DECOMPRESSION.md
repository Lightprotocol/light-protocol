# Decompression Client API

This document describes how to decompress compressed CToken ATAs and CMints.

## Quick Start

```rust
use light_compressible_client::{decompress_atas, decompress_mint};

// Decompress ATAs
let atas = vec![
    rpc.get_ata_interface(&mint, &owner).await?,
];
let instructions = decompress_atas(&atas, fee_payer, &rpc).await?;

// Decompress CMint
let mint = rpc.get_mint_interface(&signer).await?;
let instructions = decompress_mint(&mint, fee_payer, &rpc).await?;
```

## Unified Token Data

`AtaInterface` always provides `token_data` regardless of hot/cold state.
Uses the standard `TokenData` type from `light_token_sdk::compat`:

```rust
let ata = rpc.get_ata_interface(&mint, &owner).await?;

// Always works - token_data is populated from on-chain or compressed source
println!("Amount: {}", ata.token_data.amount);  // Direct field access
println!("Amount: {}", ata.amount());           // Convenience method
println!("Delegate: {:?}", ata.delegate());

// Check state
if ata.is_cold() {
    // Needs decompression
} else if ata.is_hot() {
    // Already on-chain
} else {
    // Doesn't exist
}
```

## API Overview

### ATAs

| Function                                                            | Description                                  |
| ------------------------------------------------------------------- | -------------------------------------------- |
| `decompress_atas(&[AtaInterface], fee_payer, &indexer)`             | High-perf wrapper: pre-fetch ATAs, call this |
| `build_decompress_atas(&[AtaInterface], fee_payer, proof)`          | Sync: caller provides proof                  |
| `decompress_atas_idempotent(&[(mint, owner)], fee_payer, &indexer)` | Simple: fetches everything                   |
| `rpc.get_ata_interface(&mint, &owner)`                              | Fetch ATA state with unified data            |

### CMints

| Function                                                       | Description                                  |
| -------------------------------------------------------------- | -------------------------------------------- |
| `decompress_mint(&MintInterface, fee_payer, &indexer)`         | High-perf wrapper: pre-fetch mint, call this |
| `build_decompress_mint(&MintInterface, fee_payer, proof, ...)` | Sync: caller provides proof                  |
| `decompress_mint(signer, fee_payer, &indexer)`                 | Simple: fetches everything                   |
| `rpc.get_mint_interface(&signer)`                              | Fetch CMint state                            |

## Usage Patterns

### Pattern 1: Simple (Recommended for most apps)

Fetches state and proof internally. Easy to use.

```rust
use light_compressible_client::{decompress_atas_idempotent, decompress_mint};

// Decompress ATAs by (mint, owner) pairs
let instructions = decompress_atas_idempotent(
    &[(mint1, owner1), (mint2, owner2)],
    fee_payer,
    &rpc
).await?;

// Decompress CMint by signer
let instructions = decompress_mint(signer, fee_payer, &rpc).await?;
```

### Pattern 2: High-Performance (Recommended for latency-sensitive apps)

Pre-fetch state, then call lean wrapper. Allows batching state fetches.

```rust
use light_compressible_client::{decompress_atas, decompress_mint};

// Pre-fetch ATAs (can batch with futures::join_all)
let atas = vec![
    rpc.get_ata_interface(&mint1, &owner1).await?,
    rpc.get_ata_interface(&mint2, &owner2).await?,
];

// Access data immediately (works for both hot and cold)
for ata in &atas {
    println!("ATA {} has {} tokens", ata.ata, ata.amount());
}

// Decompress cold ATAs (fetches proof internally, fast-exits if all hot)
let instructions = decompress_atas(&atas, fee_payer, &rpc).await?;

// Same for mints
let mint = rpc.get_mint_interface(&signer).await?;
let instructions = decompress_mint(&mint, fee_payer, &rpc).await?;
```

### Pattern 3: Maximum Control (For advanced use cases)

Pre-fetch state AND proof. Fully synchronous instruction building.

```rust
use light_compressible_client::{build_decompress_atas, build_decompress_mint};
use light_program_test::Indexer;

// Pre-fetch ATAs
let atas = vec![
    rpc.get_ata_interface(&mint1, &owner1).await?,
    rpc.get_ata_interface(&mint2, &owner2).await?,
];

// Check if any cold (sync, instant)
let cold_hashes: Vec<_> = atas.iter().filter_map(|a| a.hash()).collect();
if cold_hashes.is_empty() {
    return Ok(vec![]); // All hot - fast exit
}

// Get proof (async)
let proof = rpc.get_validity_proof(cold_hashes, vec![], None).await?.value;

// Build instructions (sync - no RPC)
let instructions = build_decompress_atas(&atas, fee_payer, Some(proof))?;
```

## Interface Types

### AtaInterface

```rust
pub struct AtaInterface {
    pub ata: Pubkey,           // ATA pubkey (derived)
    pub owner: Pubkey,         // Wallet owner (signer)
    pub mint: Pubkey,          // Token mint
    pub bump: u8,              // ATA bump
    pub is_cold: bool,         // Needs decompression?
    pub token_data: TokenData, // Always present (standard SPL-compatible type)
    pub raw_account: Option<Account>,  // If hot
    pub decompression: Option<DecompressionContext>,  // If cold
}

// Standard TokenData from light_token_sdk::compat (re-exported)
pub struct TokenData {
    pub mint: Pubkey,
    pub owner: Pubkey,  // Note: for ATAs, this is the ATA pubkey
    pub amount: u64,
    pub delegate: Option<Pubkey>,
    pub state: AccountState,
    pub tlv: Option<Vec<ExtensionStruct>>,
}

impl AtaInterface {
    fn is_cold(&self) -> bool;   // Needs decompression?
    fn is_hot(&self) -> bool;    // Already on-chain?
    fn is_none(&self) -> bool;   // Doesn't exist?
    fn amount(&self) -> u64;     // Convenience accessor
    fn delegate(&self) -> Option<Pubkey>;  // Convenience accessor
    fn hash(&self) -> Option<[u8; 32]>;  // For proof (if cold)
}
```

### MintInterface

```rust
pub struct MintInterface {
    pub cmint: Pubkey,              // CMint PDA
    pub signer: Pubkey,             // Mint signer (seed)
    pub address_tree: Pubkey,       // Address tree
    pub compressed_address: [u8; 32],  // Compressed address
    pub state: MintState,           // Hot/Cold/None
}

impl MintInterface {
    fn is_cold(&self) -> bool;
    fn is_hot(&self) -> bool;
    fn hash(&self) -> Option<[u8; 32]>;
}
```

## Idempotency

All functions are idempotent:

- Returns empty `Vec<Instruction>` if account is already on-chain (hot)
- Safe to call multiple times
- No errors for already-decompressed accounts

## Example: Full Decompression Flow

```rust
use light_compressible_client::{decompress_atas, decompress_mint};

async fn decompress_all(
    rpc: &mut LightProgramTest,
    signer: Pubkey,
    mint: Pubkey,
    owners: &[Pubkey],
    fee_payer: Pubkey,
    payer: &Keypair,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Decompress CMint first (required for ATA decompression)
    let mint_interface = rpc.get_mint_interface(&signer).await?;
    if mint_interface.is_cold() {
        let ix = decompress_mint(&mint_interface, fee_payer, rpc).await?;
        if !ix.is_empty() {
            rpc.create_and_send_transaction(&ix, &fee_payer, &[payer]).await?;
        }
    }

    // 2. Fetch all ATAs (can batch)
    let mut atas = Vec::new();
    for owner in owners {
        let ata = rpc.get_ata_interface(&mint, owner).await?;
        // Data is always available
        println!("Owner {} has {} tokens (cold={})", owner, ata.amount(), ata.is_cold());
        atas.push(ata);
    }

    // 3. Decompress cold ATAs
    let ix = decompress_atas(&atas, fee_payer, rpc).await?;
    if !ix.is_empty() {
        rpc.create_and_send_transaction(&ix, &fee_payer, &[payer]).await?;
    }

    Ok(())
}
```
