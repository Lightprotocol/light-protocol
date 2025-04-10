## [0.30.0]

#### Breaking Changes

This release has several breaking changes which are necessary for protocol
scalability. Please reach out to the [team](https://t.me/swen_light) if you need help migrating.

-   new type: TokenPoolInfo
-   Instruction Changes:

    -   `compress`, `mintTo`, `approveAndMintTo`, `compressSplTokenAccount` now require valid TokenPoolInfo
    -   `decompress` now requires array of one or more TokenPoolInfos.

-   Action Changes:
    -   Removed optional tokenProgramId: PublicKey
    -   removed optional merkleTree: PublicKey
    -   removed optional outputStateTree: PublicKey
    -   added optional stateTreeInfo: StateTreeInfo
    -   added optional tokenPoolInfo: TokenPoolInfo

### Migration guide: Compress

```typescript
// ...

// new code
const treeInfos = await rpc.getCachedActiveStateTreeInfos();
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
const treeInfos = await rpc.getCachedActiveStateTreeInfos();
const treeInfo = selectStateTreeInfo(treeInfos);

const infos = await getTokenPoolInfos(rpc, mint);
const selectedInfos = selectTokenPoolInfosForDecompression(
    tokenPoolInfos,
    amount,
);

const ix = await CompressedTokenProgram.decompress({
    // ...
    outputStateTreeInfo,
    tokenPoolInfos: selectedTokenPoolInfos,
});
```

## [0.20.5-0.20.9] - 2025-02-24

### Changed

-   improved documentation and error messages.

## [0.20.4] - 2025-02-19

### Breaking Changes

-   `selectMinCompressedTokenAccountsForTransfer` and
    `selectSmartCompressedTokenAccountsForTransfer` now throw an error
    if not enough accounts are found. In most cases this is not a breaking
    change, because a proof request would fail anyway. This just makes the error
    message more informative.

### Added

-   `selectSmartCompressedTokenAccountsForTransfer` and
    `selectSmartCompressedTokenAccountsForTransferorPartial`

### Changed

-   `selectMinCompressedTokenAccountsForTransfer` and
    `selectMinCompressedTokenAccountsForTransferorPartial` now accept an optional
    `maxInputs` parameter, defaulting to 4.

### Security

-   N/A

For previous release notes, check:
https://www.zkcompression.com/release-notes/1.0.0-mainnet-beta
