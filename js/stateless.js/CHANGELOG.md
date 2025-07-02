# Changelog

## [0.22.0] - 2025-06-16

### Breaking Changes

(stateless.js) SOL transfers with zkcompression don't accept `stateTreeInfo` as param anymore.

```typescript
// old
await transfer(
    connection,
    fromKeypair,
    1,
    fromKeypair,
    fromKeypair.publicKey,
    stateTreeInfo,
    {
        skipPreflight: false,
    },
);

// new
await transfer(connection, fromKeypair, 1, fromKeypair, fromKeypair.publicKey, {
    skipPreflight: false,
});
```

## [0.21.0] - 2025-04-08

This release has several breaking changes which are necessary for protocol
scalability.

Please reach out to the [team](https://t.me/swen_light) if you need help
migrating.

### Breaking changes

- Renamed `ActiveTreeBundle` to `StateTreeInfo`
- Updated `StateTreeInfo` internal structure: `{ tree: PublicKey, queue: PublicKey, cpiContext: PublicKey | null, treeType: TreeType }`
- Replaced `pickRandomTreeAndQueue` with `selectStateTreeInfo`
- Use `selectStateTreeInfo` for tree selection instead of `pickRandomTreeAndQueue`

### Deprecations

- `rpc.getValidityProof` is now deprecated, use `rpc.getValidityProofV0` instead.
- `CompressedProof` and `CompressedProofWithContext` were renamed to `ValidityProof` and `ValidityProofWithContext`

### Migration Guide

1. Update Type References if you use them:

```typescript
// Old code
const bundle: ActiveTreeBundle = {
    tree: publicKey,
    queue: publicKey,
    cpiContext: publicKey,
};

// New code
const info: StateTreeInfo = {
    tree: publicKey,
    queue: publicKey, // Now required
    cpiContext: publicKey,
    treeType: TreeType.StateV1, // New required field
};
```

2. Update Method Calls:

```typescript
// Old code
const ix = LightSystemProgram.compress({
    outputStateTree: bundle,
});

// New code
const ix = LightSystemProgram.compress({
    outputStateTree: info,
});
```

3. Tree Fetching & Selection:

```typescript
// Old code
const bundles = await connection.getCachedActiveStateTreeInfo();
const { tree, queue } = pickRandomTreeAndQueue(bundles);

// New code
const infos = await rpc.getStateTreeInfos();
const selectedInfo = selectStateTreeInfo(info);
```

4. RPC Changes:

```typescript
// Old code
// Still works, but will do one additional RPC call.
//  const proof = await rpc.getValidityProof(hash[], address[]);
const proof = await rpc.getValidityProofV0(
    inputAccounts.map(account => (bn(account.hash)),
);

// New code
// const proof = await rpc.getValidityProofV0(HashWithTree[], AddressWithTree[]);
const proof = await rpc.getValidityProofV0(
    inputAccounts.map(account => ({
        hash: account.hash,
        tree: account.treeInfo.tree,
        queue: account.treeInfo.queue,
    })),
);
```

```typescript
// new type
export interface HashWithTree {
    hash: BN254;
    tree: PublicKey;
    queue: PublicKey;
}
```

5. Other breaking changes:

**MerkleContext, MerkleContextWithMerkleProof, CompressedAccountWithMerkleContext** now use `treeInfo`:

```typescript
/**
 * Context for compressed account stored in a state tree
 */
export type MerkleContext = {
    /**
     * Tree info
     */
    treeInfo: StateTreeInfo;
    /**
     * Poseidon hash of the account. Stored as leaf in state tree
     */
    hash: BN;
    /**
     * Position of `hash` in the State tree
     */
    leafIndex: number;
    /**
     * Whether the account can be proven by index or by merkle proof
     */
    proveByIndex: boolean;
};
```

## [0.20.5-0.20.9] - 2025-02-24

### Bumped to latest compressed-token sdk

## [0.20.3] - 2025-02-19

Fixed a bug where we lose precision on token amounts if compressed token accounts are created with > Number.MAX_SAFE_INTEGER.

## [0.20.0] - 2025-01-22

### Breaking Changes

- ActiveTreeBundle is now a tuple of `tree`, `queue`, `cpiContext`, and `treeType`. `treeType` is a new enum ensuring forward compatibility.
- Updated LUT addresses for Mainnet and Devnet:
    - stateTreeLookupTableMainnet = '7i86eQs3GSqHjN47WdWLTCGMW6gde1q96G2EVnUyK2st';
    - nullifiedStateTreeLookupTableMainnet = 'H9QD4u1fG7KmkAzn2tDXhheushxFe1EcrjGGyEFXeMqT';
    - stateTreeLookupTableDevnet = '8n8rH2bFRVA6cSGNDpgqcKHCndbFCT1bXxAQG89ejVsh';
    - nullifiedStateTreeLookupTableDevnet = '5dhaJLBjnVBQFErr8oiCJmcVsx3Zj6xDekGB2zULPsnP';

### Changed

- `createRpc` can now also be called with only the `rpcEndpoint` parameter. In
  this case, `compressionApiEndpoint` and `proverEndpoint` will default to the
  same value. If no parameters are provided, default localnet values are used.

## [0.19.0] - 2025-01-20

### Breaking Changes

- Instruction methods (eg `LightSystemProgram.createAccount` and `CompressedTokenProgram.mintTo`) now require an explicit output state tree pubkey or input account, otherwise they will throw an error.

### Added

- Multiple State Tree support. Allows you to pass non-default state tree pubkeys to actions and instructions. Comes out of the box with public state trees.
    - `pickRandomStateTreeAndQueue`
    - `getLightStateTreeInfo`

- createMint allows passing of freezeAuthority in action

### Changed

- `createMint`action now lets you pass tokenprogramId explicitly. is backward compatible with boolean flag for t22.

### Deprecated

- `rpc.getValidityProof`. Now does another rpc round trip to fetch tree info. use `rpc.getValidityProofV0` and pass tree info explicitly instead.

### Security

- N/A

For previous release notes, check: https://www.zkcompression.com/release-notes/1.0.0-mainnet-beta
