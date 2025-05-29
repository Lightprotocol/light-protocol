# Light Compressed Token Client

A Rust client library for interacting with the Compressed Token Program on Solana.

## Overview

- **Compress**: Compress SPL tokens to a recipient.
- **Decompress**: Decompress compressed tokens back to an SPL token account
- **Batch Compress**: Compress SPL tokens to multiple recipients in a single instruction.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
light-compressed-token-client = "0.1.0"
```

## Usage

### Compress to one recipient

```rust
use light_compressed_token_client::{compress, CompressParams};
use solana_pubkey::Pubkey;

let instruction = compress(
    payer,
    owner,
    source_token_account,
    mint,
    1000, // amount
    recipient,
    output_state_tree,
)?;
```

### Compress to multiple recipients

```rust
use light_compressed_token_client::batch_compress;

// Compress to multiple recipients
let recipients = vec![
    (recipient1, 500),
    (recipient2, 300),
    (recipient3, 200),
];

let instruction = batch_compress(
    payer,
    owner,
    source_token_account,
    mint,
    recipients,
    output_state_tree,
)?;
```

### Decompress to SPL token account.

```rust
use light_compressed_token_client::{DecompressParams, create_decompress_instruction};

let params = DecompressParams {
    payer,
    input_compressed_token_accounts: vec![(account, token_data, merkle_context)],
    to_address: destination_token_account, // SPL token account, eg ATA.
    amount: 1000,
    recent_input_state_root_indices: vec![Some(0)], // fetch via get_validity_proof
    recent_validity_proof: Some(proof),  // fetch via get_validity_proof
    output_state_tree: Some(state_tree),
    token_program_id: None, // defaults to SPL Token
};

let instruction = create_decompress_instruction(params)?;
```

## Features

- Supports SPL-token and Token-22 mints.

## License

Apache-2.0
