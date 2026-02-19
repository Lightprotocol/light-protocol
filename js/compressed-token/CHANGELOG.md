## [0.23.0-beta.10]

### Breaking Changes

- **`decompressInterface` removed.** Use `loadAta` (action) or `createLoadAtaInstructions` (instruction builder) instead. `decompressInterface` did not support >8 compressed inputs and has been fully removed.
    - **Action (send transaction):** Replace `decompressInterface(rpc, payer, owner, mint, amount?, destinationAta?, destinationOwner?, splInterfaceInfo?, confirmOptions?)` with `loadAta(rpc, ata, owner, mint, payer?, confirmOptions?, interfaceOptions?, wrap?)`. Derive the target ATA with `getAssociatedTokenAddressInterface(mint, owner)` for c-token, or pass the SPL/T22 ATA to decompress to that program. `loadAta` loads all cold balance into the given ATA (no partial amount); it supports >8 inputs via batched transactions and creates the ATA if needed.
    - **Instruction-level:** Use `createLoadAtaInstructions(rpc, ata, owner, mint, payer?, interfaceOptions?, wrap?)` to get `TransactionInstruction[][]` and send batches yourself. The single-instruction primitive is no longer exported; use the batched API only.

## [0.23.0-beta.9]

### Fixed

- `maxTopUp` default changed from `0` (no top-ups allowed) to `MAX_TOP_UP` (65535, no cap) across all instruction builders (`wrap`, `unwrap`, `mintTo`, `createMint`, `decompressMint`, `updateMetadata`, `updateMintAuthority`). Previously rent top-ups were silently blocked, causing transaction failures on underfunded compressible accounts.
- `getSplOrToken2022AccountInterface` now fetches hot and cold accounts in parallel; individual fetch failures are handled gracefully instead of throwing immediately.
- `delegatedAmount` correctly parsed from CompressedOnly TLV extension instead of defaulting to 0.
- `parseCTokenHot` uses `unpackAccountSPL` for correct hot c-token account parsing.
- Frozen `ctoken-hot` sources now correctly excluded from load paths (was only filtering SPL/T22 frozen sources).
- SPL interface fetch errors in load paths are now rethrown when there is an SPL or T22 balance; previously all errors were silently swallowed.

### Added

- `MAX_TOP_UP` constant (65535) — exported from constants.
- `maxTopUp` optional parameter on `createWrapInstruction`, `createUnwrapInstruction`, `createMintToInstruction`, `createMintInstruction`, `decompressMintInstruction`, `createUpdateMetadataFieldInstruction`, `createUpdateMetadataAuthorityInstruction`, `createRemoveMetadataKeyInstruction`, `createUpdateMintAuthorityInstruction`, `createUpdateFreezeAuthorityInstruction` for explicit rent top-up capping.
- `maxTopUp` optional parameter on `mintToCompressed`, `unwrap`, `decompressMint` actions. **Note:** `mintToCompressed` inserts `maxTopUp` before the existing `confirmOptions` positional parameter — callers who were passing `confirmOptions` positionally must update their call sites (TypeScript will report a type error).
- `createUnwrapInstructions` — instruction builder for unwrapping, returns `TransactionInstruction[][]` with amount-aware input selection.
- `selectInputsForAmount` — greedy amount-aware compressed account selection for load/unwrap.
- `assertV2Only` guards on `loadAta` and decompress paths — V1 inputs are rejected early with a clear error.

## [0.23.0-beta.7] - Transfer Interface Hardening

### Breaking Changes

#### Renames

- **`CTOKEN_PROGRAM_ID`**: Deprecated. Use `LIGHT_TOKEN_PROGRAM_ID` (re-exported from `@lightprotocol/stateless.js`).

- **`createCTokenTransferInstruction`**: Renamed to `createLightTokenTransferInstruction`. Instruction data layout changed (see below).

- **`createTransferInterfaceInstruction`** (multi-program dispatcher): Deprecated. Use `createLightTokenTransferInstruction` for Light token transfers, or SPL's `createTransferCheckedInstruction` for SPL/T22 transfers.

#### `transferInterface` (high-level action)

- **`destination` parameter changed from ATA address to wallet public key.** The function now derives the recipient ATA internally and creates it idempotently (no extra RPC fetch). Callers that previously passed a pre-derived ATA address must now pass the recipient's wallet public key instead.

- **`programId` default changed** from `CTOKEN_PROGRAM_ID` to `LIGHT_TOKEN_PROGRAM_ID`. Parameter order unchanged: `amount, programId?, confirmOptions?, options?, wrap?`.

- **Multi-transaction support**: For >8 compressed inputs, the action now sends parallel load transactions before the final transfer transaction. Previously, all instructions were packed into a single transaction (which could exceed limits).

#### `createTransferInterfaceInstructions` (instruction builder -- NEW)

New function replacing the old monolithic `transferInterface` internals. Takes `recipient` as a wallet public key (not ATA). Returns `TransactionInstruction[][]` where each inner array is one transaction. The last element is always the transfer transaction; all preceding elements are load transactions that can be sent in parallel.

```typescript
const batches = await createTransferInterfaceInstructions(
    rpc, payer, mint, amount, sender, recipientWallet, options?,
);
const { rest: loads, last: transferTx } = sliceLast(batches);
```

Options include `ensureRecipientAta` (default: `true`) which prepends an idempotent ATA creation instruction to the transfer transaction, and `programId` which dispatches to SPL `transferChecked` for `TOKEN_PROGRAM_ID`/`TOKEN_2022_PROGRAM_ID`.

#### `createLoadAtaInstructions`

- **Return type changed** from `TransactionInstruction[]` (flat) to `TransactionInstruction[][]` (batched). Each inner array is one transaction. For >8 compressed inputs, multiple transactions are needed because each decompress proof can handle at most 8 inputs.

    ```typescript
    // Old
    const ixs: TransactionInstruction[] = await createLoadAtaInstructions(
        rpc,
        ata,
        owner,
        mint,
    );

    // New
    const batches: TransactionInstruction[][] = await createLoadAtaInstructions(
        rpc,
        ata,
        owner,
        mint,
    );
    // Each element is one transaction's instructions
    ```

#### `createLightTokenTransferInstruction` (instruction-level)

- **Instruction data layout changed**: Old format was 10 bytes (discriminator + padding + u64 LE at offset 2). New format is 9 bytes (discriminator + u64 LE at offset 1, no padding).

- **Account keys changed**: Now always includes `system_program` (index 3) and `fee_payer` (index 4) for compressible extension rent top-ups. Old format had 3 required accounts (source, destination, owner) with optional payer. New format has 5 required accounts.

- **`owner` is now writable** (for rent top-ups via compressible extension).

#### `createDecompressInterfaceInstruction`

- **New required parameter**: `decimals: number` added after `splInterfaceInfo`. Required for SPL destination decompression.

- **Delegate handling**: Now includes delegate pubkeys from input compressed accounts in the packed accounts list.

#### Program instruction: createTokenPool → createSplInterface

- **`CompressedTokenProgram.createTokenPool`**: Deprecated. Use `CompressedTokenProgram.createSplInterface` with the same call signature (`feePayer`, `mint`, `tokenProgramId?`). The high-level action `createSplInterface()` now calls the new instruction helper; the deprecated action alias `createTokenPool` still works but points to `createSplInterface`. `CompressedTokenProgram.createMint` now uses `createSplInterface` internally for the third instruction.

### Added

- **`createTransferInterfaceInstructions`**: Instruction builder for transfers with multi-transaction batching, frozen account pre-checks, zero-amount rejection, and `programId`-based dispatch (Light token vs SPL `transferChecked`).
- **`sliceLast`** helper: Splits instruction batches into `{ rest, last }` for parallel-then-sequential sending.
- **`TransferOptions`** interface: `wrap`, `programId`, `ensureRecipientAta`, extends `InterfaceOptions`.
- **Version-aware proof chunking**: V1 inputs chunked with sizes {8,4,2,1}, V2 with {8,7,6,5,4,3,2,1}. V1 and V2 never mixed in a single proof request.
- **`assertUniqueInputHashes`**: Runtime enforcement that no compressed account hash appears in more than one parallel batch.
- **`chunkAccountsByTreeVersion`**: Exported utility for splitting compressed accounts by tree version into prover-compatible groups.
- **Frozen account handling**: `_buildLoadBatches` skips frozen sources. `createTransferInterfaceInstructions` throws early if hot account is frozen, reports frozen balance in insufficient-balance errors.
- **`loadAta` action**: Now sends all load batches in parallel (previously sequential single-tx).
- **`createUnwrapInstructions`**: New instruction builder for unwrapping c-tokens to SPL/T22. Returns `TransactionInstruction[][]` (load batches, if any, then one unwrap batch). Same loop pattern as `createLoadAtaInstructions` and `createTransferInterfaceInstructions`. The `unwrap` action now uses it internally. Use this when you need instruction-level control or to handle multi-batch load + unwrap in one go.
- **`LightTokenProgram`**: Export alias for `CompressedTokenProgram` for clearer naming in docs and examples.
- **Decompress mint as part of create mint**: `createMintInterface` and the create-mint instruction now decompress the mint in the same transaction. The mint is available on-chain (CMint account created) immediately after creation; a separate `decompressMint()` call is no longer required before creating ATAs or minting. `decompressMint()` remains supported and is idempotent: if the mint was already decompressed (e.g. via `createMintInterface`), it returns successfully without sending a transaction.

## [0.22.0]

- `CreateMint` action now allows passing a non-payer mint and freeze authority.
- More efficient computebudgets for actions.
- Better DX: Parameter lookup in call signatures of CompressedTokenProgram instructions
- QoL: improved typedocs.

## [0.21.0]

#### Breaking Changes

This release has several breaking changes which improve protocol
scalability. Please reach out to the [team](https://t.me/swen_light) if you need help migrating.

### Migration guide: Compress

**Old Code** (remove this)

```typescript
const activeStateTrees = await connection.getCachedActiveStateTreeInfo();
const { tree } = pickRandomTreeAndQueue(activeStateTrees);
// ...
```

**New Code**

```typescript
// ...
const treeInfos = await rpc.getStateTreeInfos();
const treeInfo = selectStateTreeInfo(treeInfos);

const infos = await getTokenPoolInfos(rpc, mint);
const info = selectTokenPoolInfo(infos);

const compressIx = await CompressedTokenProgram.compress({
    // ...
    outputStateTreeInfo,
    tokenPoolInfo,
});
```

### Migration guide: Decompress

```typescript
// ...
// new code:
const infos = await getTokenPoolInfos(rpc, mint);
const selectedInfos = selectTokenPoolInfosForDecompression(
    tokenPoolInfos,
    amount,
);

const ix = await CompressedTokenProgram.decompress({
    // ...
    tokenPoolInfos: selectedTokenPoolInfos,
});
```

### Overview

- new type: TokenPoolInfo
- Instruction Changes:
    - `compress`, `mintTo`, `approveAndMintTo`, `compressSplTokenAccount` now require valid TokenPoolInfo
    - `decompress` now requires an array of one or more TokenPoolInfos.
    - `decompress`, `transfer` now do not allow state tree overrides.

- Action Changes:
    - Removed optional tokenProgramId: PublicKey
    - removed optional merkleTree: PublicKey
    - removed optional outputStateTree: PublicKey
    - added optional stateTreeInfo: StateTreeInfo
    - added optional tokenPoolInfo: TokenPoolInfo

- new instructions:
    - `approve`, `revoke`: delegated transfer support.
    - `addTokenPools`: you can now register additional token pool pdas. Use
      this if you need very high concurrency.

### Why the Changes are helpful

`getStateTreeInfos()` retrieves relevant info about all active state trees.

When building a transaction you can now pick a random treeInfo via `selectStateTreeInfo(infos)`.

This lets you and other apps execute more transactions within Solana's write lock
limits.

The same applies to `getTokenPoolInfos`. When you compress or decompress SPL
tokens, a tokenpool gets write-locked. If you need additional per-block write
lock capacity, you can register and sample additional (up to 4) tokenpool
accounts.

## [0.20.5-0.20.9] - 2025-02-24

### Changed

- improved documentation and error messages.

## [0.20.4] - 2025-02-19

### Breaking Changes

- `selectMinCompressedTokenAccountsForTransfer` and
  `selectSmartCompressedTokenAccountsForTransfer` now throw an error
  if not enough accounts are found. In most cases this is not a breaking
  change, because a proof request would fail anyway. This just makes the error
  message more informative.

### Added

- `selectSmartCompressedTokenAccountsForTransfer` and
  `selectSmartCompressedTokenAccountsForTransferOrPartial`

### Changed

- `selectMinCompressedTokenAccountsForTransfer` and
  `selectMinCompressedTokenAccountsForTransferorPartial` now accept an optional
  `maxInputs` parameter, defaulting to 4.

### Security

- N/A

For previous release notes, check:
https://www.zkcompression.com/release-notes/1.0.0-mainnet-beta
