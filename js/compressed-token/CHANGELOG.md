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
