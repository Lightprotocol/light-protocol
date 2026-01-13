## Compressible macros — caller program usage (first draft)

Use this to add rent-free PDAs, cTokens, and cMints to your program with minimal boilerplate.

### What you get (the interface)

- `#[derive(Compressible)]`: makes a struct compressible. Expect a `compression_info: Option<CompressionInfo>` field.
- `#[add_compressible_instructions(...)]`: generates ready-to-use `decompress_accounts_idempotent` and `compress_accounts_idempotent` entrypoints, PDA seed derivation, and optional cToken integration.
- `#[account]`: convenience macro for Anchor accounts adding `LightHasher` + `LightDiscriminator` derives.
- Rent tools: `derive_light_rent_sponsor_pda!`, `derive_light_rent_sponsor!` for compile‑time rent sponsor constants.
- Program config helpers: `process_initialize_compression_config_checked`, `process_update_compression_config`.

### How to use — PDA only

1. Define your PDAs

```rust
use light_sdk::compressible::CompressionInfo;
use light_sdk_macros::{account, Compressible};

#[account]
#[derive(Compressible)]
pub struct UserRecord {
    pub compression_info: Option<CompressionInfo>,
    pub owner: Pubkey,
    pub name: String,
    pub score: u64,
}
```

2. Generate compress/decompress instructions with auto seeds

```rust
use light_sdk_macros::add_compressible_instructions;

#[add_compressible_instructions(
  UserRecord = ("user_record", data.owner.as_ref())
)]
#[program]
pub mod my_program {}
```

3. Initialize your compression config (one-time)

- Call the generated `initialize_compression_config` entrypoint or invoke:
  - `process_initialize_compression_config_checked(config_pda, update_authority, program_data, rent_sponsor, compression_authority, rent_config, write_top_up, address_space, bump=0, payer, system_program, program_id)`
- Inputs you must pick:
  - rent_sponsor: who receives rent when PDAs compress/close
  - compression_authority: who can compress/close your PDAs
  - rent_config + write_top_up: rent curve + write top‑up per write
  - address_space: one address tree pubkey for your PDAs

4. Use the generated entrypoints

- `decompress_accounts_idempotent(...)`
- `compress_accounts_idempotent(...)`

### How to use — mixed with cToken

1. Extend the macro with token variants

```rust
#[add_compressible_instructions(
  // PDAs
  UserRecord = ("user_record", data.owner.as_ref()),
  // Program‑owned ctoken PDA (must provide authority seeds)
  TreasuryCtoken = (is_token, "treasury_ctoken", ctx.fee_payer, authority = (ctx.treasury)),
  // User ATA variant (no seeds, derived from owner+mint)
  UserAta = (is_token, is_ata)
)]
#[program]
pub mod my_program {}
```

2. Create compressible token accounts (ATAs) on the client or via CPI

- Inputs (client builder): `CreateCompressibleAssociatedTokenAccountInputs { payer, owner, mint, compressible_config, rent_sponsor, pre_pay_num_epochs, lamports_per_write, token_account_version }`
- Authority-less user ATAs use `derive_ctoken_ata(owner, mint)` under the hood.

3. Decompress/compress flows

- The generated `decompress_accounts_idempotent` and `compress_accounts_idempotent` accept packed token data alongside your PDAs. You only provide the standard accounts the macro adds (fee_payer, config, rent_sponsor, and optional ctoken config/cpi auth).

### How to use — cMints (compressed mints)

- Create a compressed mint:
  - `create_compressed_mint(CreateCompressedMintInputs { decimals, mint_authority, freeze_authority, proof, address_merkle_tree_root_index, mint_signer, payer, address_tree_pubkey, output_queue, extensions, version })`
  - Derive addresses with:
    - `derive_mint_compressed_address(&mint_signer, &address_tree_pubkey)`
    - `find_mint_address(&mint_signer)`
- Mint tokens to compressed accounts:
  - `create_mint_to_compressed_instruction(MintToCompressedInputs { compressed_mint_inputs, recipients, mint_authority, payer, state_merkle_tree, input_queue, output_queue_cmint, output_queue_tokens, decompressed_mint_config, proof, token_account_version, cpi_context_pubkey, token_pool })`

Keep it simple: create cMint → mint to recipients (compressed accounts or cToken ATAs) using the SDK helpers below.

### cToken SDK (compressed-token-sdk) — the interfaces you actually call

- Accounts
  - `derive_ctoken_ata(owner, mint) -> (Pubkey, u8)`
  - `create_compressible_associated_token_account(inputs)` / `_idempotent` (+ “2” variants if owner/mint passed as accounts)
  - Low-level: `create_compressible_token_account_instruction(CreateCompressibleTokenAccount)`
- Mints
  - `create_compressed_mint(CreateCompressedMintInputs)`
  - `derive_mint_compressed_address(mint_seed, address_tree)`
  - `find_mint_address(mint_seed)`
- Mint to recipients
  - `create_mint_to_compressed_instruction(MintToCompressedInputs)`
  - Types: `Recipient { recipient, amount }`
- Transfer SPL ↔ cToken
  - `create_transfer_spl_to_ctoken_instruction(...)`
  - `create_transfer_ctoken_to_spl_instruction(...)`
  - `transfer_interface(...)` / `transfer_interface_signed(...)`
- Update compressed mint
  - `update_compressed_mint(UpdateCompressedMintInputs)`

### Rent — set/update for your PDAs and for cTokens

- PDAs (your program)
  - One-time config: `process_initialize_compression_config_checked(...)` (or use generated `initialize_compression_config` entrypoint)
  - Update later: `process_update_compression_config(config, authority, new_update_authority?, new_rent_sponsor?, new_compression_authority?, new_rent_config?, new_write_top_up?, new_address_space?, program_id)`
  - Use `light_compressible::rent::RentConfig` to define rent curve and distribution. Funds on close/compress go to `rent_sponsor` (completed epochs) and refund fee payer for partial epochs automatically.
- cTokens (account-level)
  - When creating a compressible token account, you pass:
    - `rent_sponsor`, `pre_pay_num_epochs`, optional `lamports_per_write`, and `compressible_config` (the registry’s or your chosen config PDA)
  - For ATAs: `CreateCompressibleAssociatedTokenAccountInputs { ... }`

### Rust client — the minimum you need

1. Connect and fetch proofs

```rust
use light_client::rpc::{LightClient, LightClientConfig, Rpc};

let mut rpc = LightClient::new(LightClientConfig::local()).await?; // or devnet/mainnet
// rpc.get_validity_proof(account_hashes, new_addresses, None).await?
```

2. Create a compressible ATA

```rust
use light_token_sdk::instructions::{
  create_compressible_associated_token_account, CreateCompressibleAssociatedTokenAccountInputs
};

let ix = create_compressible_associated_token_account(CreateCompressibleAssociatedTokenAccountInputs {
  payer,
  owner,
  mint,
  compressible_config,
  rent_sponsor,
  pre_pay_num_epochs: 2,
  lamports_per_write: Some(1_000),
  token_account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
})?;
```

3. Create a cMint and mint to recipients

```rust
use light_token_sdk::instructions::{
  create_compressed_mint, CreateCompressedMintInputs,
  create_mint_to_compressed_instruction, MintToCompressedInputs
};
use light_token_interface::instructions::mint_action::Recipient;

let create_cmint_ix = create_compressed_mint(CreateCompressedMintInputs { /* fill from RPC + keys */ })?;
let mint_ix = create_mint_to_compressed_instruction(MintToCompressedInputs {
  recipients: vec![Recipient { recipient: some_address, amount: 1000 }],
  /* queues/tree/authority from RPC + keys */
}, None)?;
```

4. High-level helpers (token-client)

```rust
use light_token_client::actions::{create_compressible_token_account, CreateCompressibleTokenAccountInputs, mint_to_compressed};

let token_acc = create_compressible_token_account(&mut rpc, CreateCompressibleTokenAccountInputs {
  owner, mint, num_prepaid_epochs: 2, payer: &payer, token_account_keypair: None,
  lamports_per_write: None, token_account_version: light_token_interface::state::TokenDataVersion::ShaFlat
}).await?;

let sig = mint_to_compressed(&mut rpc, spl_mint_pda, vec![Recipient{ recipient: token_acc, amount: 1000 }], light_token_interface::state::TokenDataVersion::ShaFlat, &mint_authority, &payer).await?;
```

### TL;DR checklists

- PDA only
  - Add `#[derive(Compressible)]` + `compression_info`
  - Add `#[add_compressible_instructions(...)]` with seeds
  - Initialize config (rent_sponsor, compression_authority, rent_config, write_top_up, address_space)
  - Call generated compress/decompress entrypoints
- Mixed with cToken
  - Add token variants in `#[add_compressible_instructions(...)]` (program-owned with `authority = (...)` or `is_ata`)
  - Use SDK to create cToken ATAs; pass rent fields
  - Mint via cMints and `mint_to_compressed` or `mint_action`
- cMints
  - `create_compressed_mint(...)` then `create_mint_to_compressed_instruction(...)`
