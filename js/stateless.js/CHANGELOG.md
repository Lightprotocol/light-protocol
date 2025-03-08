# Changelog

## [0.20.5-0.20.9] - 2025-02-24

### Bumped to latest compressed-token sdk

## [0.20.3] - 2025-02-19

Fixed a bug where we lose precision on token amounts if compressed token accounts are created with > Number.MAX_SAFE_INTEGER.

## [0.20.0] - 2025-01-22

### Breaking Changes

-   StateTreeInfo is now a tuple of `tree`, `queue`, `cpiContext`, and `treeType`. `treeType` is a new enum ensuring forward compatibility.
-   Updated LUT addresses for Mainnet and Devnet:
    -   stateTreeLookupTableMainnet = '7i86eQs3GSqHjN47WdWLTCGMW6gde1q96G2EVnUyK2st';
    -   nullifiedStateTreeLookupTableMainnet = 'H9QD4u1fG7KmkAzn2tDXhheushxFe1EcrjGGyEFXeMqT';
    -   stateTreeLookupTableDevnet = '8n8rH2bFRVA6cSGNDpgqcKHCndbFCT1bXxAQG89ejVsh';
    -   nullifiedStateTreeLookupTableDevnet = '5dhaJLBjnVBQFErr8oiCJmcVsx3Zj6xDekGB2zULPsnP';

### Changed

-   `createRpc` can now also be called with only the `rpcEndpoint` parameter. In this case, `compressionApiEndpoint` and `proverEndpoint` will default to the same value. If no parameters are provided, default localnet values are used.

## [0.19.0] - 2025-01-20

### Breaking Changes

-   Instruction methods (eg `LightSystemProgram.createAccount` and `CompressedTokenProgram.mintTo`) now require an explicit output state tree pubkey or input account, otherwise they will throw an error.

### Added

-   Multiple State Tree support. Allows you to pass non-default state tree pubkeys to actions and instructions. Comes out of the box with public state trees.

    -   `pickRandomStateTreeAndQueue`
    -   `getActiveStateTreeInfos`

-   createMint allows passing of freezeAuthority in action

### Changed

-   `createMint`action now lets you pass tokenprogramId explicitly. is backward compatible with boolean flag for t22.

### Deprecated

-   `rpc.getValidityProof`. Now does another rpc round trip to fetch tree info. use `rpc.getValidityProofV0` and pass tree info explicitly instead.

### Security

-   N/A

For previous release notes, check: https://www.zkcompression.com/release-notes/1.0.0-mainnet-beta
