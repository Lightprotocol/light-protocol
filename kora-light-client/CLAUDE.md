# kora-light-client

## Summary

- Standalone Light Protocol instruction builders for solana-sdk 3.0 consumers (Kora)
- Zero `light-*` crate dependencies — all Borsh-serializable types are duplicated locally with byte-identical serialization to the on-chain program
- Builds Solana `Instruction` structs for Transfer2, Decompress, Wrap, Unwrap, CreateATA, and TransferChecked
- Uses packed account indices (u8) with HashMap-based deduplication for compact instruction data
- Golden byte tests verify serialization compatibility with the on-chain compressed token program

## Used in

- **Kora** (external) — Solana client using solana-sdk 3.0; consumes instruction builders for compressed token operations

## Scope and limitations

**Covers:**
- Compressed-to-compressed transfers (Transfer2)
- Decompress to light-token or SPL accounts
- Wrap (SPL → light-token) and Unwrap (light-token → SPL)
- Create associated token accounts with compressible config
- Decompressed ATA-to-ATA transfers (TransferChecked)
- Greedy account selection (max 8 inputs per transaction)
- Multi-transaction batch orchestration with compute budget estimation

**Does not cover:**
- CreateMint, MintTo, MintToChecked, Freeze, Thaw, Approve, Revoke, CloseAccount, Burn
- RPC client for querying compressed accounts or fetching proofs
- Transaction signing, sending, or confirmation
- Address lookup table loading

**Caller responsibilities:**
- Fetch compressed account data from Photon indexer/RPC → `CompressedTokenAccountInput`
- Fetch validity proofs from prover server → `CompressedProof`
- Derive PDAs for pools and ATAs as needed
- Assemble instructions into versioned transactions with LUTs
- Sign and submit transactions

## Navigation

This file contains all documentation for the crate. For on-chain instruction behavior, see:
- `programs/compressed-token/program/CLAUDE.md` — program overview and instruction index
- `programs/compressed-token/program/docs/compressed_token/TRANSFER2.md` — Transfer2 on-chain processing
- `programs/compressed-token/program/docs/ctoken/CREATE.md` — CreateAssociatedTokenAccount on-chain processing
- `programs/compressed-token/program/docs/ctoken/TRANSFER_CHECKED.md` — TransferChecked on-chain processing

## Integration workflow

End-to-end flow for using this crate:

```
1. Fetch compressed accounts  → CompressedTokenAccountInput
   Source: Photon indexer / RPC
   Note: Kora implements TryFrom<CompressedTokenAccountRpc> for this type

2. Select input accounts      → select_input_accounts(accounts, target_amount)
   Returns up to 8 accounts using greedy descending selection

3. Fetch validity proof        → CompressedProof
   Source: prover server via RPC
   Note: proof is optional when all inputs use prove_by_index

4. Derive PDAs if needed
   get_associated_token_address(owner, mint)     → light-token ATA
   find_spl_interface_pda(mint)                  → SPL pool PDA (for wrap/unwrap/SPL decompress)

5. Build instruction(s)
   create_transfer2_instruction(...)             → compressed-to-compressed
   create_decompress_instruction(...)            → compressed → on-chain account
   create_wrap_instruction(...)                  → SPL → light-token
   create_unwrap_instruction(...)                → light-token → SPL
   create_ata_idempotent_instruction(...)        → create ATA
   create_transfer_checked_instruction(...)      → ATA-to-ATA

6. Set compute budget
   Use constants from load_ata.rs or create_load_ata_batches() for automatic estimation

7. Assemble transaction
   Use versioned transactions (V0) with LIGHT_LUT_MAINNET or LIGHT_LUT_DEVNET

8. Sign and send
   Caller's responsibility
```

## Address lookup tables

`LIGHT_LUT_MAINNET` and `LIGHT_LUT_DEVNET` are exported for transaction assembly. All Transfer2/Decompress instructions include 7+ static program accounts (LightSystemProgram, CpiAuthorityPDA, RegisteredProgramPDA, etc.) that benefit from LUT compression. Callers should use versioned transactions (V0) and include the relevant LUT to keep transactions within the 1232-byte limit.

Both mainnet and devnet currently point to the same address: `9NYFyEqPeWQHiS8Jv4VjZcjKBMPRCJ3KbEbaBcy4Mza`.

## Design constraints

- **Zero light-\* dependencies.** All types are ported with identical Borsh layout. This avoids version conflicts with Kora's solana-sdk 3.0 dependency tree.
- **solana-sdk 3.0 split crates.** Uses `solana-pubkey` 3.0, `solana-instruction` 3.0, `solana-system-interface` 2.0, `solana-compute-budget-interface` 3.0.
- **Borsh cross-version compatibility.** This crate uses borsh 1.5; the on-chain program uses borsh 0.10 via AnchorSerialize. The binary format is identical for these primitive types (verified by golden byte tests).
- **Packed u8 account indices.** Instruction data references accounts by u8 index into a deduplicated packed accounts array (see packed accounts scheme below).
- **Two Transfer2 layouts.** Standard (7 static accounts) for compressed inputs, decompressed-only (2 static accounts) for wrap/unwrap.

## Packed accounts scheme

All instruction builders (except CreateAta and TransferChecked) use the same pattern:

1. **Static prefix.** Fixed accounts at the start of the accounts array.
2. **Packed suffix.** Remaining accounts are deduplicated via `HashMap<Pubkey, u8>` and appended.
3. **Index references.** Instruction data uses u8 indices into the packed portion.
4. **Flag upgrades.** When a pubkey is inserted twice, `is_signer` and `is_writable` flags are OR'd (upgraded, never downgraded).

**Insert order for packed accounts:**
trees (writable) → queues (writable) → mint → authority/owner (signer) → destination → [delegates] → [pool (writable), token_program]

### Standard layout (Transfer2, Decompress)

| Index | Account | Signer | Writable |
|-------|---------|--------|----------|
| 0 | LightSystemProgram | | |
| 1 | payer | yes | yes |
| 2 | CpiAuthorityPDA | | |
| 3 | RegisteredProgramPDA | | |
| 4 | AccountCompressionAuthorityPDA | | |
| 5 | AccountCompressionProgram | | |
| 6 | SystemProgram | | |
| 7+ | packed accounts... | varies | varies |

### Decompressed-only layout (Wrap, Unwrap)

| Index | Account | Signer | Writable |
|-------|---------|--------|----------|
| 0 | CpiAuthorityPDA | | |
| 1 | payer | yes | yes |
| 2+ | packed accounts... | varies | varies |
| N-2 | LightTokenProgram | | |
| N-1 | SystemProgram | | |

Packed accounts for wrap/unwrap use fixed indices (not HashMap):
0=mint, 1=owner(signer), 2=source(writable), 3=destination(writable), 4=pool(writable), 5=token_program.

## Public API — Instruction builders

### create_transfer2_instruction

```rust
fn create_transfer2_instruction(
    payer: &Pubkey,           // fee payer (signer)
    authority: &Pubkey,       // token owner or delegate (signer)
    mint: &Pubkey,            // token mint
    inputs: &[CompressedTokenAccountInput],  // source compressed accounts
    proof: &CompressedProof,  // validity proof from RPC
    destination_owner: &Pubkey, // owner of destination compressed account
    amount: u64,              // amount to transfer
) -> Result<Instruction, KoraLightError>
```

- **discriminator:** 101 (`TRANSFER2_DISCRIMINATOR`)
- **layout:** standard (7 static + packed)
- **path:** `src/transfer.rs`

Builds a Transfer2 instruction for compressed-to-compressed token transfers. Automatically creates a change output if `amount < input_total`. Omits the proof from instruction data when all inputs use `prove_by_index`.

Signers: payer, authority.

### create_decompress_instruction

```rust
fn create_decompress_instruction(
    payer: &Pubkey,
    owner: &Pubkey,
    mint: &Pubkey,
    inputs: &[CompressedTokenAccountInput],
    proof: &CompressedProof,
    destination: &Pubkey,     // on-chain token account (light-token ATA or SPL ATA)
    amount: u64,
    decimals: u8,
    spl_interface: Option<&SplInterfaceInfo>, // None for light-token, Some for SPL
) -> Result<Instruction, KoraLightError>
```

- **discriminator:** 101 (Transfer2 with `Compression::Decompress`)
- **layout:** standard (7 static + packed)
- **path:** `src/decompress.rs`

Routes between light-token decompress (no pool, `spl_interface=None`) and SPL decompress (with pool account and token program added to packed accounts). Creates a change output if `amount < input_total`.

Signers: payer, owner. Packed accounts include pool (writable) and token_program when `spl_interface` is provided.

### create_wrap_instruction

```rust
fn create_wrap_instruction(
    source: &Pubkey,          // SPL token account (writable)
    destination: &Pubkey,     // light-token account (writable)
    owner: &Pubkey,           // token owner (signer)
    mint: &Pubkey,
    amount: u64,
    decimals: u8,
    payer: &Pubkey,           // fee payer (signer)
    spl_interface: &SplInterfaceInfo,
) -> Result<Instruction, KoraLightError>
```

- **discriminator:** 101 (Transfer2 with two compressions)
- **layout:** decompressed-only (2 static + fixed packed)
- **path:** `src/wrap.rs`

Uses two compression operations: `Compress(SPL)` moves tokens from SPL source to pool, then `Decompress(light-token)` moves them from pool to light-token destination. No compressed inputs or outputs (empty vecs), no proof needed.

Signers: payer, owner. Total accounts: 10 (2 static + 6 packed + 2 appended programs).

### create_unwrap_instruction

```rust
fn create_unwrap_instruction(
    source: &Pubkey,          // light-token account (writable)
    destination: &Pubkey,     // SPL token account (writable)
    owner: &Pubkey,           // token owner (signer)
    mint: &Pubkey,
    amount: u64,
    decimals: u8,
    payer: &Pubkey,           // fee payer (signer)
    spl_interface: &SplInterfaceInfo,
) -> Result<Instruction, KoraLightError>
```

- **discriminator:** 101 (Transfer2 with two compressions)
- **layout:** decompressed-only (2 static + fixed packed)
- **path:** `src/unwrap.rs`

Reverse of wrap: `Compress(light-token)` then `Decompress(SPL)`. Same account layout and structure as wrap with different compression modes.

### CreateAta / create_ata_idempotent_instruction

```rust
struct CreateAta {
    payer: Pubkey,
    owner: Pubkey,
    mint: Pubkey,
    idempotent: bool,                // default: false
    compressible_config: Pubkey,     // default: LIGHT_TOKEN_CONFIG
    rent_sponsor: Pubkey,            // default: RENT_SPONSOR_V1
    pre_pay_num_epochs: u8,          // default: 16
    write_top_up: u32,               // default: 766 lamports
    compression_only: bool,          // default: true
}

// Builder usage
CreateAta::new(payer, owner, mint).idempotent().instruction()

// Convenience function
fn create_ata_idempotent_instruction(payer, owner, mint) -> Result<Instruction>
```

- **discriminator:** 100 (CreateATA) or 102 (CreateATA idempotent)
- **path:** `src/create_ata.rs`

Accounts (7, fixed order):

| Index | Account | Signer | Writable |
|-------|---------|--------|----------|
| 0 | owner | | |
| 1 | mint | | |
| 2 | payer | yes | yes |
| 3 | ATA (derived) | | yes |
| 4 | SystemProgram | | |
| 5 | compressible_config | | |
| 6 | rent_sponsor | | yes |

ATA address is derived from `get_associated_token_address(owner, mint)`.

### create_transfer_checked_instruction

```rust
fn create_transfer_checked_instruction(
    source_ata: &Pubkey,
    destination_ata: &Pubkey,
    mint: &Pubkey,
    owner: &Pubkey,           // signer
    amount: u64,
    decimals: u8,
    payer: &Pubkey,           // signer, only added if != owner
) -> Result<Instruction, KoraLightError>
```

- **discriminator:** 12
- **path:** `src/transfer.rs`

For decompressed (on-chain) light-token ATA-to-ATA transfers. Not for compressed accounts. Data format: discriminator(1) + amount(8 LE) + decimals(1) = 10 bytes.

Accounts: source(writable), mint, destination(writable), owner(signer), SystemProgram, [payer(signer) if payer != owner].

## Public API — Utilities

### select_input_accounts

```rust
fn select_input_accounts(
    accounts: &[CompressedTokenAccountInput],
    target_amount: u64,
) -> Result<Vec<CompressedTokenAccountInput>, KoraLightError>
```

- **path:** `src/account_select.rs`
- **constant:** `MAX_INPUT_ACCOUNTS = 8`

Greedy descending selection: sorts accounts by amount (largest first), selects minimum accounts to meet target. Returns up to 8 accounts. Returns empty vec if `target_amount = 0`. Errors on empty input, insufficient balance, or arithmetic overflow.

### create_load_ata_batches

```rust
fn create_load_ata_batches(input: LoadAtaInput) -> Result<Vec<LoadBatch>, KoraLightError>
```

- **path:** `src/load_ata.rs`

Orchestrates multi-transaction decompress flows. Chunks compressed accounts into batches of 8 (`MAX_INPUT_ACCOUNTS`). Each batch is one transaction containing:
- Compute budget instruction (auto-calculated)
- ATA creation (first batch, or idempotent in subsequent batches)
- Optional wrap instruction
- Decompress instruction for the chunk

Input types:

- `LoadAtaInput` — payer, owner, mint, decimals, destination, needs_ata_creation, compressed_accounts, proofs (one per chunk), spl_interface, spl_wrap
- `LoadBatch` — instructions, num_compressed_accounts, has_ata_creation, wrap_count
- `WrapSource` — source_ata, amount, spl_interface

Validates that `proofs.len() == chunks.len()`.

## Compute budget guidance

For callers not using `create_load_ata_batches` (which handles this automatically):

| Component | Compute units |
|-----------|--------------|
| ATA creation | 30,000 |
| Wrap operation | 50,000 |
| Decompress base | 50,000 |
| Full ZK proof verification | 100,000 |
| Per account (prove-by-index) | 10,000 |
| Per account (full proof) | 30,000 |

**Formula:** `(base + per_account × N) × 1.3`, clamped to [50,000 .. 1,400,000].

Example: decompress 4 accounts with full proof = `(50K + 100K + 4 × 30K) × 1.3 = 351K CU`.

Constants are defined in `src/load_ata.rs`.

## PDA derivation

**path:** `src/pda.rs`

| Function | Seeds | Program |
|----------|-------|---------|
| `get_associated_token_address(owner, mint)` | [owner, LIGHT_TOKEN_PROGRAM_ID, mint] | LIGHT_TOKEN_PROGRAM_ID |
| `get_associated_token_address_and_bump(owner, mint)` | same as above | same |
| `find_spl_interface_pda(mint)` | [b"pool", mint, 0] | LIGHT_TOKEN_PROGRAM_ID |
| `find_spl_interface_pda_with_index(mint, pool_index)` | [b"pool", mint, pool_index] | LIGHT_TOKEN_PROGRAM_ID |
| `derive_cpi_authority_pda()` | [b"cpi_authority"] | LIGHT_TOKEN_PROGRAM_ID |

`is_light_token_owner(owner)` — returns `Some(true)` for LIGHT_TOKEN_PROGRAM_ID, `Some(false)` for SPL Token or Token-2022, `None` otherwise.

## Types

### On-chain mirror types (Borsh-serializable)

All types must remain byte-identical to the on-chain program. Verified by golden byte tests.

| Type | Size (bytes) | Ported from |
|------|-------------|-------------|
| `CompressedProof` | 128 | `program-libs/compressed-account/src/instruction_data/compressed_proof.rs` |
| `PackedMerkleContext` | 7 | `program-libs/compressed-account/src/compressed_account.rs` |
| `CompressedCpiContext` | 2 | `program-libs/token-interface/src/instructions/transfer2/cpi_context.rs` |
| `CompressionMode` (enum) | 1 | `program-libs/token-interface/src/instructions/transfer2/compression.rs` |
| `Compression` | 16 | same as above |
| `MultiInputTokenDataWithContext` | 22 | `program-libs/token-interface/src/instructions/transfer2/instruction_data.rs` |
| `MultiTokenTransferOutputData` | 13 | same as above |
| `CompressedTokenInstructionDataTransfer2` | variable | same as above |
| `ExtensionInstructionData` (enum, 33 variants) | variable | `program-libs/token-interface/src/instructions/extensions/` |
| `TokenMetadataInstructionData` | variable | same as above (variant 19) |
| `CompressedOnlyExtensionInstructionData` | 21 | same as above (variant 31) |
| `CompressionInfo` | 80 | `program-libs/compressible/` (variant 32) |
| `CreateAssociatedTokenAccountInstructionData` | variable | `program-libs/token-interface/src/instructions/create_associated_token_account.rs` |
| `CompressibleExtensionInstructionData` | variable | `program-libs/token-interface/src/instructions/extensions/compressible.rs` |
| `CompressToPubkey` | variable | same as above |
| `AdditionalMetadata` | variable | key-value pair for token metadata |

### Client-only types (not serialized on-chain)

| Type | Purpose |
|------|---------|
| `CompressedTokenAccountInput` | Compressed account data from RPC, ready for instruction building. Kora implements `TryFrom<CompressedTokenAccountRpc>`. |
| `SplInterfaceInfo` | SPL pool info (PDA, bump, pool_index, token_program) for compress/decompress via SPL. |
| `ValidityProofWithContext` | Proof + root indices from RPC. Root indices are per-input, same order. |
| `LoadAtaInput` | Pre-fetched data for `create_load_ata_batches`. |
| `LoadBatch` | One transaction's worth of instructions from batch orchestration. |
| `WrapSource` | SPL balance to wrap as part of a load operation. |

## Constants

**path:** `src/program_ids.rs`

### Program IDs

| Constant | Value |
|----------|-------|
| `LIGHT_TOKEN_PROGRAM_ID` | `cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m` |
| `LIGHT_SYSTEM_PROGRAM_ID` | `SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7` |
| `ACCOUNT_COMPRESSION_PROGRAM_ID` | `compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq` |
| `SPL_TOKEN_PROGRAM_ID` | `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA` |
| `SPL_TOKEN_2022_PROGRAM_ID` | `TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb` |
| `SYSTEM_PROGRAM_ID` | `11111111111111111111111111111111` |
| `NOOP_PROGRAM_ID` | `noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV` |

### Pre-derived PDAs

| Constant | Value |
|----------|-------|
| `CPI_AUTHORITY_PDA` | `GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy` (bump 254) |
| `ACCOUNT_COMPRESSION_AUTHORITY_PDA` | `HwXnGK3tPkkVY6P439H2p68AxpeuWXd5PcrAxFpbmfbA` |
| `REGISTERED_PROGRAM_PDA` | `35hkDgaAKwMCaxRz2ocSZ6NaUrtKkyNqU6c4RV3tYJRh` |
| `LIGHT_TOKEN_CONFIG` | `ACXg8a7VaqecBWrSbdu73W4Pg9gsqXJ3EXAqkHyhvVXg` |
| `RENT_SPONSOR_V1` | `r18WwUxfG8kQ69bQPAB2jV6zGNKy3GosFGctjQoV4ti` |

### Other constants

| Constant | Value | Purpose |
|----------|-------|---------|
| `TRANSFER2_DISCRIMINATOR` | `101` | Transfer2 instruction discriminator |
| `TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR` | `[2,0,0,0,0,0,0,0]` | Compressed token account discriminator |
| `WSOL_MINT` | `So11111111111111111111111111111111111111112` | Wrapped SOL mint |
| `CPI_AUTHORITY_PDA_SEED` | `b"cpi_authority"` | Seed for CPI authority derivation |
| `BUMP_CPI_AUTHORITY` | `254` | Known bump for CPI authority PDA |
| `POOL_SEED` | `b"pool"` | Seed for SPL pool PDA derivation |
| `NUM_MAX_POOL_ACCOUNTS` | `5` | Maximum pool accounts per mint |
| `LIGHT_LUT_MAINNET` | `9NYFyEqPeWQHiS8Jv4VjZcjKBMPRCJ3KbEbaBcy4Mza` | Mainnet address lookup table |
| `LIGHT_LUT_DEVNET` | `9NYFyEqPeWQHiS8Jv4VjZcjKBMPRCJ3KbEbaBcy4Mza` | Devnet address lookup table |

## Errors

**path:** `src/error.rs`

| Variant | Description | Common cause |
|---------|-------------|--------------|
| `CannotDetermineAccountType` | Owner pubkey is not LIGHT_TOKEN_PROGRAM_ID, SPL Token, or Token-2022 | Passing an unknown program as account owner to `is_light_token_owner` |
| `InsufficientBalance { needed, available }` | Input accounts don't cover requested amount | `select_input_accounts` or builders with amount > sum of inputs |
| `NoCompressedAccounts` | Empty inputs slice passed to builder | Calling a builder or `select_input_accounts` with `&[]` |
| `BorshError(io::Error)` | Borsh serialization failure | Corrupted data or internal serialization bug |
| `ArithmeticOverflow` | Checked arithmetic failed | Extremely large token amounts that overflow u64 |
| `InvalidInput(String)` | General validation failure | `create_load_ata_batches` with mismatched proof/chunk count |

## Source code structure

### Instruction builders

| File | Lines | Description |
|------|-------|-------------|
| `src/transfer.rs` | 416 | Transfer2 (compressed-to-compressed) and TransferChecked (ATA-to-ATA) |
| `src/decompress.rs` | 523 | Decompress via Transfer2 with Compression operation |
| `src/wrap.rs` | 153 | SPL → light-token via dual-compression Transfer2 (decompressed_accounts_only layout) |
| `src/unwrap.rs` | 187 | Light-token → SPL via dual-compression Transfer2 (decompressed_accounts_only layout) |
| `src/create_ata.rs` | 183 | CreateAssociatedTokenAccount builder with compressible config |

### Utilities

| File | Lines | Description |
|------|-------|-------------|
| `src/account_select.rs` | 161 | Greedy descending account selection (max 8, `MAX_INPUT_ACCOUNTS`) |
| `src/load_ata.rs` | 375 | Multi-transaction batch orchestration with compute budget estimation |

### Core

| File | Lines | Description |
|------|-------|-------------|
| `src/lib.rs` | 44 | Module declarations and re-exports |
| `src/types.rs` | 560 | All Borsh-serializable types (on-chain mirrors + client-only) |
| `src/program_ids.rs` | 82 | 31 constants (program IDs, PDAs, seeds, LUT addresses) |
| `src/pda.rs` | 78 | 6 PDA derivation functions |
| `src/error.rs` | 23 | `KoraLightError` enum (6 variants) |

### Tests

| File | Lines | Description |
|------|-------|-------------|
| `tests/golden_bytes.rs` | 382 | Borsh serialization cross-verification against on-chain format |
| `src/types.rs` (inline) | ~60 | Borsh verification gates (proof=128B, context=7B, compression=16B, input=22B, output=13B) |
| `src/` (inline per module) | ~200 | Unit tests per module (account order, deduplication, error paths, round-trips) |

## Testing

```bash
# Run from kora-light-client/ directory (crate is excluded from workspace)
cd kora-light-client && cargo test
```

### Golden byte tests (`tests/golden_bytes.rs`)

8 tests that verify byte-identical serialization with the on-chain program:

1. `test_transfer2_header_matches_kora_format` — header serialization (150 bytes with empty vecs)
2. `test_input_token_data_matches_kora_format` — 22 bytes per input
3. `test_output_token_data_on_chain_format` — 13 bytes per output (see compatibility note below)
4. `test_full_instruction_data_format` — discriminator + complete struct
5. `test_compression_serialization` — 16 bytes per Compression struct
6. `test_compressed_only_extension_serialization` — 21 bytes
7. `test_extension_enum_discriminators` — variants 0, 31, 32
8. `test_transfer2_roundtrip` — serialize → deserialize identity

### Borsh verification gates (`src/types.rs`)

6 inline tests verifying individual type sizes match on-chain expectations.

## Compatibility and version pinning

**Source version:** `types.rs` header says "Source commit: HEAD of main branch at time of porting" — no pinned commit hash. Golden byte tests are the primary drift detection mechanism.

**12 → 13 byte output format change:** Kora's existing raw-byte builder (`instruction_builder.rs`) uses a 12-byte output format per `MultiTokenTransferOutputData`:

```
Kora old format (12 bytes):  owner(u8), amount(u64), lamports(Option<u64>=None), merkle_tree_index(u8), tlv(Option=None)
On-chain format (13 bytes):  owner(u8), amount(u64), has_delegate(bool), delegate(u8), mint(u8), version(u8)
```

This crate uses the 13-byte on-chain format. When Kora adopts this crate, its output format will change. If the deployed on-chain program uses an older format, this needs investigation before deploying.

**Verification:** Run `cd kora-light-client && cargo test` after any upstream changes to the on-chain types. Golden byte tests will fail if serialization drifts.
