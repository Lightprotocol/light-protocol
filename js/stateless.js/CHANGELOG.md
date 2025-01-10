# Changelog

## [0.19.0] - 2025-01-20

### Breaking Changes

-   Instruction methods (eg `LightSystemProgram.createAccount` and `CompressedTokenProgram.mintTo`) now require an explicit output state tree pubkey or input account, otherwise they will throw an error.

### Added

-   Multiple State Tree support. Allows you to pass non-default state tree pubkeys to actions and instructions. Comes out of the box with public state trees.

    -   `pickRandomStateTreeAndQueue`
    -   `getLightStateTreeInfo`

-   createMint allows passing of freezeAuthority in action

### Changed

-   `createMint`action now lets you pass tokenprogramId explicitly. is backward compatible with boolean flag for t22.

### Deprecated

-   `rpc.getValidityProof`. Now does another rpc round trip to fetch tree info. use `rpc.getValidityProofV0` and pass tree info explicitly instead.

### Security

-   N/A

For previous release notes, check: https://www.zkcompression.com/release-notes/1.0.0-mainnet-beta
